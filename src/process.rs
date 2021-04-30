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
    process::{Child, ChildStderr, ChildStdout, Command},
    signal, task, time,
};

use crate::{Cmd, Error, Location, Result};

lazy_static! {
    /// Default timeout (in seconds) that is used when a [`Process`](Process) is cretaed via [`process!`](process!) macro without providing a specific timeout.

    /// It can be configured by setting `PROCESS_TIMEOUT` environment variable.
    pub static ref TIMEOUT: u64 = {
        let default = 10;
        match std::env::var("PROCESS_TIMEOUT") {
            Err(_) => default,
            Ok(timeout) => match timeout.parse::<u64>() {
                Ok(x) => x,
                Err(_) => {
                    eprintln!(
                        "⚠️  TIMEOUT variable is not a valid int: {}. Using default: {}",
                        timeout, default
                    );
                    default
                }
            },
        }
    };
}

/// Long running process. Can be constructed via [`Process::new`](Process::new) or convenience [`process!`](process!) macro.
pub struct Process<Loc> {
    /// Tag used as an identificator in output when process runs as a part of a [`ProcessPool`](ProcessPool).
    pub tag: &'static str,
    /// [Command](Cmd) to run a process.
    pub cmd: Cmd<Loc>,
    /// Amount of time to wait before killing hanged process. See also [`TIMEOUT`](struct@TIMEOUT).
    pub timeout: Duration,
}

enum TeardownReason {
    CtrlC,
    ProcessFinished(io::Result<Output>),
}

enum CtrlCResult {
    ProcessExited,
    Timeout,
}

pub(crate) enum ExitResult {
    Output(Output),
    Interrupted,
    Killed { pid: u32 },
}

impl<Loc> Process<Loc>
where
    Loc: Location + Send + Sync,
{
    /// Constructs a new process.
    pub fn new(tag: &'static str, cmd: Cmd<Loc>, timeout: Duration) -> Self {
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
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

/// Convenience macro for creating a [`Process`](Process).
///
/// ## Examples
/// Constructing a process with the default [`TIMEOUT`](struct@TIMEOUT):
/// ```rust
/// process! {
///   tag: "server",
///   cmd: cmd! { ... },
/// }
/// ```
///
/// Constructing a process with the specific timeout:
/// ```rust
/// use std::time::Duration;
///
/// process! {
///   tag: "server",
///   cmd: cmd! { ... },
///   timeout: Duration::from_secs(20),
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
            std::time::Duration::from_secs(*$crate::process::TIMEOUT),
        )
    };
}

pub(crate) struct RunningProcess {
    process: Child,
    timeout: Duration,
}

impl RunningProcess {
    pub(crate) async fn spawn<Loc>(
        cmd: &Cmd<Loc>,
        stdout: Stdio,
        stderr: Stdio,
        timeout: Duration,
    ) -> io::Result<Self>
    where
        Loc: Location + Send + Sync,
    {
        let process = Command::new(Cmd::<Loc>::SHELL)
            .args(Cmd::<Loc>::shelled(&cmd.exe))
            .envs(cmd.env.to_owned())
            .current_dir(cmd.pwd.as_path())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()?;

        Ok(Self { process, timeout })
    }

    pub(crate) fn stdout(&mut self) -> Option<ChildStdout> {
        self.process.stdout.take()
    }

    pub(crate) fn stderr(&mut self) -> Option<ChildStderr> {
        self.process.stderr.take()
    }

    pub(crate) async fn wait(self) -> Result<ExitResult> {
        let process = self.process;
        let pid = process.id().expect("wait is called on exited process"); // failing fast
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
                      result.unwrap_or_else(|err| Err(io::Error::new(io::ErrorKind::Other, err)))
                    ),
                _ = signal::ctrl_c() => TeardownReason::CtrlC,
            }
        };

        match exit_reason {
            TeardownReason::ProcessFinished(result) => {
                let output = result?;
                if output.status.success() {
                    Ok(ExitResult::Output(output))
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
                        _ = time::sleep(self.timeout) => CtrlCResult::Timeout,
                    }
                };

                match res {
                    CtrlCResult::ProcessExited => Ok(ExitResult::Interrupted),
                    CtrlCResult::Timeout => match Self::kill(pid) {
                        Ok(()) => Ok(ExitResult::Killed { pid }),
                        Err(err) => Err(err),
                    },
                }
            }
        }
    }

    #[cfg(unix)]
    pub fn kill(pid: u32) -> nix::Result<()> {
        use nix::{
            sys::signal::{self, Signal},
            unistd::Pid,
        };

        let pid = Pid::from_raw(pid as i32);
        signal::kill(pid, Signal::SIGKILL)
    }

    #[cfg(windows)]
    pub fn kill(pid: u32) -> Result<()> {
        use winapi::{
            shared::{
                minwindef::{BOOL, DWORD, FALSE, UINT},
                ntdef::NULL
            },
            um::{
                handleapi::CloseHandle,
                errhandlingapi::GetLastError,
                processthreadsapi::{OpenProcess, TerminateProcess},
                winnt::{PROCESS_TERMINATE, HANDLE},
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

            Err(Error::Zombie {
                pid,
                err,
            })
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

/// Struct to run a pool of long riunning processes.
///
/// ```rust
/// ProcessPool::run(vec![ process_1, process_2 ]).await
/// ```
pub struct ProcessPool;

impl ProcessPool {
    /// Runs a pool of long riunning processes.
    pub async fn run<Loc>(pool: Vec<Process<Loc>>) -> Result<()>
    where
        Loc: Location + Send + Sync + 'static,
    {
        let pool_size = pool.len();
        let exited_processes = Arc::new(AtomicUsize::new(0));

        let tag_col_length = pool.iter().fold(0, |acc, process| {
            let len = process.tag().len();
            if len > acc {
                len
            } else {
                acc
            }
        });

        let timeout = pool.iter().fold(Duration::from_secs(0), |x, p| {
            if p.timeout > x {
                p.timeout
            } else {
                x
            }
        });

        let colors = colors::make(pool_size as u8);
        let processes: Vec<(Process<Loc>, Color)> = pool.into_iter().zip(colors).collect();

        let processes_list = processes
            .iter()
            .fold(String::new(), |acc, (process, color)| {
                let styled = console::style(process.tag().to_string()).fg(*color).bold();
                if acc == "" {
                    styled.to_string()
                } else {
                    format!("{}, {}", acc, styled)
                }
            });

        eprintln!("❯ {} {}", console::style("Running:").bold(), processes_list);

        for (process, color) in processes {
            let exited_processes = exited_processes.clone();

            task::spawn(async move {
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

                eprintln!(
                    "{tag} {headline}",
                    tag = colored_tag_col,
                    headline = crate::headline!(cmd),
                );

                let mut process: RunningProcess =
                    RunningProcess::spawn(cmd, Stdio::piped(), Stdio::piped(), timeout)
                        .await
                        .expect(&format!("Failed to spawn {} process", colored_tag))
                        .into();

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
                    Ok(ExitResult::Output(_)) => eprintln!(
                        "{} Process {} exited with code 0.",
                        colored_tag_col, colored_tag
                    ),
                     Ok(ExitResult::Interrupted) => eprintln!(
                        "{} Process {} successfully exited.",
                        colored_tag_col, colored_tag
                    ),
                    Ok(ExitResult::Killed { pid }) => eprintln!(
                        "{} Process {} with pid {} was killed due to timeout.",
                        colored_tag_col,
                        colored_tag,
                        pid = pid,
                    ),
                    Err(Error::NonZeroExitCode { code, output: _ }) => eprintln!(
                        "{} Process {} exited with non-zero code: {:#?}",
                        colored_tag_col, colored_tag, code
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

                exited_processes.fetch_add(1, Ordering::Relaxed);
            });
        }

        signal::ctrl_c().await.unwrap();

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
    fn process_macro_with_timeout<Loc: Location + Send + Sync>(cmd: Cmd<Loc>) -> Process<Loc> {
        process! {
          tag: "server",
          cmd: cmd,
          timeout: Duration::from_secs(20),
        }
    }

    #[allow(dead_code)]
    fn process_macro_without_timeout<Loc: Location + Send + Sync>(cmd: Cmd<Loc>) -> Process<Loc> {
        process! {
          tag: "server",
          cmd: cmd,
        }
    }
}
