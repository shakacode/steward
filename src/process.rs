use std::{
    io,
    process::{Output, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use console::Color;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout},
    signal, task, time,
};

use crate::{Cmd, Dependency, Error, KillTimeout, Location, Result, SpawnOptions};

/// Long running process. Can be constructed via [`Process::new`](Process::new) or convenience [`process!`](crate::process!) macro.
pub struct Process<Loc> {
    /// Tag used as an identificator in output when process runs as a part of a [`ProcessPool`](ProcessPool).
    pub tag: &'static str,
    /// [Command](Cmd) to run a process.
    pub cmd: Cmd<Loc>,
    /// Amount of time to wait before killing hanged process. See [`KillTimeout`](crate::KillTimeout).
    pub timeout: KillTimeout,
}

enum TeardownReason {
    CtrlC,
    ProcessFinished(io::Result<Output>),
}

enum CtrlCResult {
    ProcessExited,
    Timeout,
}

// pub(crate) enum ExitResult {
//     Output(Output),
//     Interrupted,
//     Killed { pid: u32 },
// }

impl<Loc> Process<Loc>
where
    Loc: Location,
{
    /// Constructs a new process.
    pub fn new(tag: &'static str, cmd: Cmd<Loc>, timeout: KillTimeout) -> Self {
        Self { tag, cmd, timeout }
    }

    /// Returns a tag of a process.
    pub fn tag(&self) -> &'static str {
        self.tag
    }

    /// Returns a command of a process.
    pub fn cmd(&self) -> &Cmd<Loc> {
        &self.cmd
    }

    /// Returns a timeout of a process.
    pub fn timeout(&self) -> &KillTimeout {
        &self.timeout
    }

    /// Spawns a process and returns a [`RunningProcess`](RunningProcess),
    /// which includes a [`Child`](tokio::process::Child).
    pub async fn spawn(&self, opts: SpawnOptions) -> io::Result<RunningProcess> {
        self.cmd().spawn(opts)
    }
}

/// Convenience macro for creating a [`Process`](Process).
///
/// ## Examples
/// Constructing a process with the default [`KillTimeout`](crate::KillTimeout):
/// ```ignore
/// process! {
///   tag: "server",
///   cmd: cmd! { ... },
/// }
/// ```
///
/// Constructing a process with the specific timeout:
/// ```ignore
/// use std::time::Duration;
///
/// process! {
///   tag: "server",
///   cmd: cmd! { ... },
///   timeout: Duration::from_secs(20).into(),
/// }
/// ```
#[macro_export]
macro_rules! process {
    {
        tag: $tag:expr,
        cmd: $cmd:expr,
        timeout: $timeout:expr$(,)?
    } => {
        $crate::Process::new(
            $tag,
            $cmd,
            $timeout,
        )
    };
    {
        tag: $tag:expr,
        cmd: $cmd:expr$(,)?
    } => {
        $crate::Process::new(
            $tag,
            $cmd,
            $crate::KillTimeout::default(),
        )
    };
}

/// Wrapper around a running child process.
pub struct RunningProcess {
    pub(crate) process: Child,
    pub(crate) timeout: KillTimeout,
    pub(crate) group: bool,
}

impl RunningProcess {
    /// Returns a reference to the underlying [`Child`](tokio::process::Child) process.
    pub fn as_child(&self) -> &Child {
        &self.process
    }

    /// Consumes the instance and gives a handle to the underlying [`Child`](tokio::process::Child) process.
    pub fn into_child(self) -> Child {
        self.process
    }

    pub(crate) fn stdout(&mut self) -> Option<ChildStdout> {
        self.process.stdout.take()
    }

    pub(crate) fn stderr(&mut self) -> Option<ChildStderr> {
        self.process.stderr.take()
    }

    pub(crate) async fn wait(self) -> Result<Output> {
        let process = self.process;

        let pid = match process.id() {
            Some(pid) => pid,
            None => return Err(Error::ProcessDoesNotExist),
        };

        let process_exited = Arc::new(AtomicBool::new(false));

        let exit_reason = {
            let process_exited = process_exited.clone();

            let process_task = task::spawn(async move {
                let res = process.wait_with_output().await;
                process_exited.store(true, Ordering::SeqCst);
                res
            });

            tokio::select! {
                result =
                  process_task =>
                    TeardownReason::ProcessFinished(
                      result.unwrap_or_else(|err| Err(io::Error::other(err)))
                    ),
                _ = signal::ctrl_c() => TeardownReason::CtrlC,
            }
        };

        match exit_reason {
            TeardownReason::ProcessFinished(result) => {
                let output = result?;
                if output.status.success() {
                    Ok(output)
                } else {
                    Err(output.into())
                }
            }
            TeardownReason::CtrlC => {
                let res = {
                    let process_exited = process_exited.clone();
                    let exit_checker = task::spawn(async move {
                        loop {
                            if process_exited.load(Ordering::SeqCst) {
                                break;
                            }
                        }
                    });
                    tokio::select! {
                        _ = exit_checker => CtrlCResult::ProcessExited,
                        _ = time::sleep(*self.timeout) => CtrlCResult::Timeout,
                    }
                };

                match res {
                    CtrlCResult::ProcessExited => Err(Error::Interrupted),
                    CtrlCResult::Timeout => match Self::kill(pid) {
                        Ok(()) => Err(Error::Killed { pid }),
                        Err(err) => Err(err),
                    },
                }
            }
        }
    }

    /// Tries to safely terminate a running process. If the termination didn't succeed, tries to kill it.
    #[cfg(unix)]
    pub async fn stop(mut self) -> Result<()> {
        use nix::{
            sys::signal::{self, Signal},
            unistd::Pid,
        };

        match self.process.id() {
            None => Err(Error::ProcessDoesNotExist),
            Some(pid) => {
                // When group is true, use negative PID to signal the entire process group
                let target = Pid::from_raw(if self.group {
                    -(pid as i32)
                } else {
                    pid as i32
                });

                match signal::kill(target, Signal::SIGINT) {
                    Ok(()) => {
                        let process = &mut self.process;

                        let res = tokio::select! {
                            res = process.wait() => Some(res),
                            _ = time::sleep(*self.timeout) => None,
                        };

                        match res {
                            Some(Ok(_)) => Ok(()),
                            Some(Err(error)) => {
                                eprintln!(
                                    "⚠️ IO error on SIGINT: {error}. Killing the process {pid}."
                                );
                                signal::kill(target, Signal::SIGKILL)
                                    .map_err(|err| Error::Zombie { pid, err })
                            }
                            None => {
                                eprintln!("⚠️ SIGINT timeout. Killing the process {pid}.");
                                signal::kill(target, Signal::SIGKILL)
                                    .map_err(|err| Error::Zombie { pid, err })
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("⚠️ Failed to terminate the process {pid}. {error}. Killing it.");
                        signal::kill(target, Signal::SIGKILL)
                            .map_err(|err| Error::Zombie { pid, err })
                    }
                }
            }
        }
    }

    // TODO: Implement RunningProcess::stop for windows

    #[cfg(unix)]
    pub(crate) fn kill(pid: u32) -> Result<()> {
        use nix::{
            sys::signal::{self, Signal},
            unistd::Pid,
        };

        signal::kill(Pid::from_raw(pid as i32), Signal::SIGKILL)
            .map_err(|err| Error::Zombie { pid, err })
    }

    #[cfg(windows)]
    pub(crate) fn kill(pid: u32) -> Result<()> {
        use winapi::{
            shared::{
                minwindef::{BOOL, DWORD, FALSE, UINT},
                ntdef::NULL,
            },
            um::{
                errhandlingapi::GetLastError,
                handleapi::CloseHandle,
                processthreadsapi::{OpenProcess, TerminateProcess},
                winnt::{HANDLE, PROCESS_TERMINATE},
            },
        };

        // since we only wish to kill the process
        const DESIRED_ACCESS: DWORD = PROCESS_TERMINATE;

        const INHERIT_HANDLE: BOOL = FALSE;

        // for some reason windows doesn't have any exit codes,
        // you just use what ever you want?
        //
        // so we're using exit code `0` then
        const EXIT_CODE: UINT = 0;

        // windows being window you have to call this a lot
        // so i just extracted it to its own function
        unsafe fn get_error(pid: u32) -> Result<()> {
            // https://docs.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
            let err: DWORD = GetLastError();

            Err(Error::Zombie { pid, err })
        }

        unsafe {
            // https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess
            let handle: HANDLE = OpenProcess(DESIRED_ACCESS, INHERIT_HANDLE, pid);
            if handle == NULL {
                get_error(pid)?;
            }

            // https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess
            let terminate_result: BOOL = TerminateProcess(handle, EXIT_CODE);
            if terminate_result == FALSE {
                get_error(pid)?;
            }

            // https://docs.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle
            let close_result: BOOL = CloseHandle(handle);
            if close_result == FALSE {
                get_error(pid)?;
            }
        }

        Ok(())
    }
}

/// Entry of a [`ProcessPool`](ProcessPool) when some of the processes depend on something.
/// It is used as an input to the [`ProcessPool::run_with_deps`](ProcessPool::run_with_deps) method.
/// See [`dep`](crate::dep) module documentation.
pub enum PoolEntry<Loc, Dep: ?Sized> {
    /// An indipendent long-running process.
    Process(Process<Loc>),
    /// A long-running process that depends on some other thing.
    ProcessWithDep {
        /// The process.
        process: Process<Loc>,
        /// The dependency. See [`Dependency`](Dependency).
        dependency: Box<Dep>,
    },
}

impl<Loc> PoolEntry<Loc, dyn Dependency>
where
    Loc: Location + 'static,
{
    fn process(&self) -> &Process<Loc> {
        match self {
            Self::Process(process) => process,
            Self::ProcessWithDep {
                process,
                dependency: _,
            } => process,
        }
    }

    fn take(self) -> (Process<Loc>, Option<Box<dyn Dependency>>) {
        match self {
            Self::Process(process) => (process, None),
            Self::ProcessWithDep {
                process,
                dependency,
            } => (process, Some(dependency)),
        }
    }
}

/// Struct to run a pool of long-running processes.
///
/// ```ignore
/// ProcessPool::run(vec![process_1, process_2]).await
/// ```
pub struct ProcessPool;

impl ProcessPool {
    /// Runs a pool of long-running processes.
    pub async fn run<Loc>(pool: Vec<Process<Loc>>) -> Result<()>
    where
        Loc: Location + 'static,
    {
        let pool = pool.into_iter().map(|p| PoolEntry::Process(p)).collect();
        ProcessPool::runner::<Loc>(pool).await
    }

    /// Runs a pool of long-running processes, some of which depend on something,
    /// such as an HTTP service being available or a file existing.
    /// See [`dep`](crate::dep) module documentation.
    pub async fn run_with_deps<Loc>(pool: Vec<PoolEntry<Loc, dyn Dependency>>) -> Result<()>
    where
        Loc: Location + 'static,
    {
        ProcessPool::runner(pool).await
    }

    async fn runner<Loc>(pool: Vec<PoolEntry<Loc, dyn Dependency>>) -> Result<()>
    where
        Loc: Location + 'static,
    {
        let pool_size = pool.len();
        let exited_processes = Arc::new(AtomicUsize::new(0));

        let (tag_col_length, timeout) =
            pool.iter()
                .fold((0, Duration::default()), |(len, timeout), entry| {
                    let process = entry.process();
                    let len = {
                        let tag_len = process.tag().len();
                        if tag_len > len {
                            tag_len
                        } else {
                            len
                        }
                    };
                    let timeout = if *process.timeout > timeout {
                        *process.timeout
                    } else {
                        timeout
                    };
                    (len, timeout)
                });

        let colors = colors::make(pool_size as u8);
        let processes: Vec<(PoolEntry<Loc, dyn Dependency>, Color)> =
            pool.into_iter().zip(colors).collect();

        let processes_list = processes.iter().fold(String::new(), |acc, (entry, color)| {
            let process = entry.process();
            let styled = console::style(process.tag().to_string()).fg(*color).bold();
            if acc.is_empty() {
                styled.to_string()
            } else {
                format!("{}, {}", acc, styled)
            }
        });

        eprintln!("❯ {} {}", console::style("Running:").bold(), processes_list);

        for (entry, color) in processes {
            let exited_processes = exited_processes.clone();

            task::spawn(async move {
                let (process, dependency) = entry.take();
                let tag = process.tag();
                let cmd = process.cmd();
                let timeout = process.timeout();
                let colored_tag = console::style(tag.to_owned()).fg(color).bold();
                let colored_tag_col = {
                    let len = tag.len();
                    let pad = " ".repeat(if len < tag_col_length {
                        tag_col_length - len + 2
                    } else {
                        2
                    });
                    console::style(format!(
                        "{tag}{pad}{pipe}",
                        tag = colored_tag,
                        pad = pad,
                        pipe = console::style("|").fg(color).bold()
                    ))
                };

                let dep_res = match dependency {
                    None => Ok(()),
                    Some(dependency) => {
                        let dep_tag = console::style(dependency.tag()).bold();

                        eprintln!(
                            "{col} {process} is waiting for its {dep} dependency...",
                            col = colored_tag_col,
                            dep = dep_tag,
                            process = colored_tag
                        );

                        let res = dependency.wait().await;
                        if let Err(error) = &res {
                            eprintln!(
                                "{col} ❗️ {dep} dependency of {process} errored: {error}\nNot executing {process}.",
                                col = colored_tag_col,
                                dep = dep_tag,
                                process = colored_tag,
                                error = error
                            );
                        }
                        res
                    }
                };

                if let Ok(()) = dep_res {
                    eprintln!(
                        "{tag} {headline}",
                        tag = colored_tag_col,
                        headline = crate::headline!(cmd),
                    );

                    let opts = SpawnOptions {
                        stdout: Stdio::piped(),
                        stderr: Stdio::piped(),
                        timeout: timeout.to_owned(),
                        ..Default::default()
                    };

                    let mut process = process.spawn(opts).await.unwrap_or_else(|err| {
                        panic!("Failed to spawn {} process. {}", colored_tag, err)
                    });

                    match process.stdout() {
                        None => eprintln!(
                            "{} Unable to read from {} stdout",
                            colored_tag_col, colored_tag
                        ),
                        Some(stdout) => {
                            let mut reader = BufReader::new(stdout).lines();
                            task::spawn({
                                let tag = colored_tag_col.clone();
                                async move {
                                    while let Some(line) = reader.next_line().await.unwrap() {
                                        eprintln!("{} {}", tag, line);
                                    }
                                }
                            });
                        }
                    }

                    match process.stderr() {
                        None => eprintln!(
                            "{} Unable to read from {} stderr",
                            colored_tag_col, colored_tag
                        ),
                        Some(stderr) => {
                            let mut reader = BufReader::new(stderr).lines();
                            task::spawn({
                                let tag = colored_tag_col.clone();
                                async move {
                                    while let Some(line) = reader.next_line().await.unwrap() {
                                        eprintln!("{} {}", tag, line);
                                    }
                                }
                            });
                        }
                    }

                    let res = process.wait().await;

                    match res {
                        Ok(_) => eprintln!(
                            "{} Process {} exited with code 0.",
                            colored_tag_col, colored_tag
                        ),
                         Err(Error::Interrupted) => eprintln!(
                            "{} Process {} successfully exited.",
                            colored_tag_col, colored_tag
                        ),
                        Err(Error::Killed { pid }) => eprintln!(
                            "{} Process {} with pid {pid} was killed due to timeout.",
                            colored_tag_col,
                            colored_tag,
                        ),
                        Err(Error::NonZeroExitCode { code, output: _ }) => eprintln!(
                            "{} Process {} exited with non-zero code: {}",
                            colored_tag_col,
                            colored_tag,
                            code.map(|x| format!("{}", x)).unwrap_or_else(|| "-".to_string())
                        ),
                        Err(Error::ProcessDoesNotExist) => eprintln!(
                            "{} ⚠️  Process {} does not exist.",
                            colored_tag_col, colored_tag
                        ),
                        Err(Error::Zombie { pid, err }) => eprintln!(
                            "{} ⚠️  Process {} with pid {} hanged and we were unable to kill it. Error: {}",
                            colored_tag_col, colored_tag, pid, err
                        ),
                        Err(Error::IoError(err)) => eprintln!(
                            "{} Process {} exited with error: {}",
                            colored_tag_col, colored_tag, err
                        ),
                    }
                }

                exited_processes.fetch_add(1, Ordering::Relaxed);
            });
        }

        signal::ctrl_c().await.unwrap();
        eprintln!(); // Prints `^C` in terminal on its own line

        let expire = Instant::now() + timeout;
        while exited_processes.load(Ordering::Relaxed) < pool_size {
            if Instant::now() > expire {
                eprintln!("⚠️  Timeout. Exiting.");
                break;
            }
            time::sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }
}

mod colors {
    use console::Color;
    use rand::{seq::SliceRandom, thread_rng};

    pub fn make(n: u8) -> Vec<Color> {
        // Preferred colors
        let mut primaries = vec![
            // Color::Red, // Red is for errors
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Magenta,
            Color::Cyan,
        ];
        // Not as good as primaries, but good enough to distinct processes
        let secondaries = vec![
            Color::Color256(24),
            Color::Color256(172),
            Color::Color256(142),
        ];

        // Let's check first if we can get away with just primary colors
        if n <= primaries.len() as u8 {
            shuffle(primaries, n)
        }
        // Otherwise, let's check if primary + secondary combined would work
        else if n <= (primaries.len() + primaries.len()) as u8 {
            primaries.extend(secondaries);
            shuffle(primaries, n)
        } else {
            // TODO: Duplicate primary + secondary colors vec as many is needed, then shuffle
            todo!()
        }
    }

    fn shuffle<T>(mut items: Vec<T>, n: u8) -> Vec<T> {
        items.truncate(n as usize);
        items.shuffle(&mut thread_rng());
        items
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{Cmd, Location, Process};

    #[allow(dead_code)]
    fn process_macro_with_timeout<Loc: Location>(cmd: Cmd<Loc>) -> Process<Loc> {
        process! {
          tag: "server",
          cmd: cmd,
          timeout: Duration::from_secs(20).into(),
        }
    }

    #[allow(dead_code)]
    fn process_macro_without_timeout<Loc: Location>(cmd: Cmd<Loc>) -> Process<Loc> {
        process! {
          tag: "server",
          cmd: cmd,
        }
    }
}
