use std::{
    io,
    ops::Deref,
    process::{self, Stdio},
    time::Duration,
};

use once_cell::sync::Lazy;
use tokio::process::Command;

use crate::{Env, ExitResult, Location, Result, RunningProcess};

/// Struct holds a specification of a command. Can be used for running one-off commands, long running processes etc.
#[derive(Clone)]
pub struct Cmd<Loc> {
    /// Command to run.
    pub exe: String,
    /// Environment of a process.
    pub env: Env,
    /// Working directory of a process.
    pub pwd: Loc,
    /// Message displayed when running a command.
    pub msg: Option<String>,
}

impl<Loc> Cmd<Loc>
where
    Loc: Location,
{
    /// Command to run.
    pub fn exe(&self) -> &str {
        &self.exe
    }

    /// Environment of a process.
    pub fn env(&self) -> &Env {
        &self.env
    }

    /// Working directory of a process.
    pub fn pwd(&self) -> &Loc {
        &self.pwd
    }

    /// Message displayed when running a command.
    pub fn msg(&self) -> Option<&String> {
        self.msg.as_ref()
    }
}

/// Amount of time to wait before killing hanged process.
///
/// When constructing a new [`Process`](crate::Process) via [`process!`](crate::process!) macro
/// without providing a specific timeout, the [`KillTimeout::default`](KillTimeout::default) implementation is used.
/// By default, the timeout is 10 seconds, but it can be configured by setting `PROCESS_TIMEOUT` environment variable.
#[derive(Clone, Debug)]
pub struct KillTimeout(Duration);

impl KillTimeout {
    /// Constructs a new timeout.
    pub fn new(duration: Duration) -> Self {
        Self(duration)
    }

    /// Returns underlying [`Duration`](std::time::Duration).
    pub fn duration(&self) -> Duration {
        self.0
    }
}

static DEFAULT_KILL_TIMEOUT: Lazy<Duration> = Lazy::new(|| {
    let default = Duration::from_secs(10);
    match std::env::var("PROCESS_TIMEOUT") {
        Err(_) => default,
        Ok(timeout) => match timeout.parse::<u64>() {
            Ok(x) => Duration::from_secs(x),
            Err(_) => {
                eprintln!(
                    "⚠️  TIMEOUT variable is not a valid int: {}. Using default: {}",
                    timeout,
                    default.as_secs()
                );
                default
            }
        },
    }
});

impl Default for KillTimeout {
    fn default() -> Self {
        Self(*DEFAULT_KILL_TIMEOUT)
    }
}

impl Deref for KillTimeout {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Duration> for KillTimeout {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

/// Options for [`Cmd::spawn`](Cmd::spawn).
pub struct SpawnOptions {
    /// Stdout stream.
    pub stdout: Stdio,
    /// Stderr stream.
    pub stderr: Stdio,
    /// Amount of time to wait before killing hanged process. See [`KillTimeout`](crate::KillTimeout).
    pub timeout: KillTimeout,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            stdout: Stdio::inherit(),
            stderr: Stdio::inherit(),
            timeout: KillTimeout::default(),
        }
    }
}

/// Enum returned from [`Cmd::output`](Cmd::output).
pub enum Output {
    /// Bytes collected from stdout.
    Data(Vec<u8>),
    /// Returned when child process has been interrupted (e.g. user pressed Ctrl + C).
    Interrupted,
}

impl Output {
    /// Returns bytes from stdout. Be aware that if child process was interrupted
    /// during the command execution (e.g. user pressed Ctrl + C), this function will terminate
    /// current process with zero exit code.
    pub fn unwrap(self) -> Vec<u8> {
        match self {
            Self::Data(bytes) => bytes,
            Self::Interrupted => process::exit(0), // not sure if this is the right thing to do
        }
    }

    /// Same as [`Output::unwrap`](Output::unwrap) but attempts to convert bytes to `String`.
    pub fn unwrap_string(self) -> Result<String> {
        let bytes = self.unwrap();
        let string = String::from_utf8(bytes)?;
        Ok(string)
    }
}

impl<Loc> Cmd<Loc>
where
    Loc: Location,
{
    #[cfg(unix)]
    pub(crate) const SHELL: &'static str = "/bin/sh";

    #[cfg(windows)]
    pub(crate) const SHELL: &'static str = "cmd";

    #[cfg(unix)]
    pub(crate) fn shelled(cmd: &str) -> Vec<&str> {
        vec!["-c", cmd]
    }

    #[cfg(windows)]
    pub(crate) fn shelled(cmd: &str) -> Vec<&str> {
        vec!["/c", cmd]
    }

    /// Runs one-off command with inherited [`Stdio`](std::process::Stdio). Prints headline (witn [`Cmd::msg`](Cmd::msg), if provided) to stderr.
    pub async fn run(&self) -> Result<()> {
        eprintln!("{}", crate::headline!(self));

        let opts = SpawnOptions {
            stdout: Stdio::inherit(),
            stderr: Stdio::inherit(),
            ..Default::default()
        };

        self.spawn(opts)?.wait().await?;

        Ok(())
    }

    /// Runs one-off command. Doesn't print anything.
    pub async fn silent(&self) -> Result<()> {
        let opts = SpawnOptions {
            stdout: Stdio::null(),
            stderr: Stdio::null(),
            ..Default::default()
        };

        self.spawn(opts)?.wait().await?;

        Ok(())
    }

    /// Runs one-off command and returns [`Output`](Output). Doesn't print anything.
    pub async fn output(&self) -> Result<Output> {
        let opts = SpawnOptions {
            stdout: Stdio::piped(),
            stderr: Stdio::piped(),
            ..Default::default()
        };

        let res = self.spawn(opts)?.wait().await?;

        match res {
            ExitResult::Output(output) => Ok(Output::Data(output.stdout)),
            ExitResult::Interrupted | ExitResult::Killed { pid: _ } => Ok(Output::Interrupted),
        }
    }

    /// A low-level method for spawning a process and getting a handle to it.
    pub fn spawn(&self, opts: SpawnOptions) -> io::Result<RunningProcess> {
        let cmd = self;

        let SpawnOptions {
            stdout,
            stderr,
            timeout,
        } = opts;

        let process = Command::new(Cmd::<Loc>::SHELL)
            .args(Cmd::<Loc>::shelled(&cmd.exe))
            .envs(cmd.env.to_owned())
            .current_dir(cmd.pwd.as_path())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()?;

        Ok(RunningProcess { process, timeout })
    }
}

/// Convenience macro for creating a [`Cmd`](Cmd).
///
/// ## Examples
/// General command:
/// ```ignore
/// cmd! {
///   exe: "rm -rf target",
///   env: Env::empty(),
///   pwd: Loc::root(),
///   msg: "Removing target dir",
/// }
/// ```
///
/// Dynamically constructed command:
/// ```ignore
/// cmd! {
///   exe: format!("rm -rf {}", dir),
///   env: Env::empty(),
///   pwd: Loc::root(),
///   msg: format!("Removing {} dir", dir),
/// }
/// ```
///
/// Command without a message:
/// ```ignore
/// cmd! {
///   exe: "ls",
///   env: Env::empty(),
///   pwd: Loc::root(),
/// }
/// ```
#[macro_export]
macro_rules! cmd {
    {
        exe: $exe:literal,
        env: $env:expr,
        pwd: $pwd:expr,
        msg: $msg:literal$(,)?
    } => {
        $crate::Cmd {
            exe: $exe.to_string(),
            env: $env,
            pwd: $pwd,
            msg: Some($msg.to_string()),
        }
    };
    {
        exe: $exe:literal,
        env: $env:expr,
        pwd: $pwd:expr,
        msg: Some($msg:expr)$(,)?
    } => {
        $crate::Cmd {
            exe: $exe.to_string(),
            env: $env,
            pwd: $pwd,
            msg: Some($msg),
        }
    };
    {
        exe: $exe:literal,
        env: $env:expr,
        pwd: $pwd:expr,
        msg: None$(,)?
    } => {
        $crate::Cmd {
            exe: $exe.to_string(),
            env: $env,
            pwd: $pwd,
            msg: None,
        }
    };
    {
        exe: $exe:literal,
        env: $env:expr,
        pwd: $pwd:expr,
        msg: $msg:expr$(,)?
    } => {
        $crate::Cmd {
            exe: $exe.to_string(),
            env: $env,
            pwd: $pwd,
            msg: Some($msg),
        }
    };
    {
        exe: $exe:expr,
        env: $env:expr,
        pwd: $pwd:expr,
        msg: $msg:literal$(,)?
    } => {
        $crate::Cmd {
            exe: $exe,
            env: $env,
            pwd: $pwd,
            msg: Some($msg.to_string()),
        }
    };
    {
        exe: $exe:expr,
        env: $env:expr,
        pwd: $pwd:expr,
        msg: Some($msg:expr)$(,)?
    } => {
        $crate::Cmd {
            exe: $exe,
            env: $env,
            pwd: $pwd,
            msg: Some($msg),
        }
    };
    {
        exe: $exe:expr,
        env: $env:expr,
        pwd: $pwd:expr,
        msg: None$(,)?
    } => {
        $crate::Cmd {
            exe: $exe,
            env: $env,
            pwd: $pwd,
            msg: None,
        }
    };
    {
        exe: $exe:expr,
        env: $env:expr,
        pwd: $pwd:expr,
        msg: $msg:expr$(,)?
    } => {
        $crate::Cmd {
            exe: $exe,
            env: $env,
            pwd: $pwd,
            msg: Some($msg),
        }
    };
    {
        exe: $exe:literal,
        env: $env:expr,
        pwd: $pwd:expr$(,)?
    } => {
        $crate::Cmd {
            exe: $exe.to_string(),
            env: $env,
            pwd: $pwd,
            msg: None,
        }
    };
    {
        exe: $exe:expr,
        env: $env:expr,
        pwd: $pwd:expr$(,)?
    } => {
        $crate::Cmd {
            exe: $exe,
            env: $env,
            pwd: $pwd,
            msg: None,
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::{Cmd, Env, Location};

    #[allow(dead_code)]
    fn cmd_macro_exe_literal_msg_literal<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: "ls",
          env: env,
          pwd: loc,
          msg: "!",
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_exe_expr_msg_literal<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: "!",
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_exe_expr_msg_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: format!("!"),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_exe_literal_msg_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: "ls",
          env: env,
          pwd: loc,
          msg: format!("!"),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_exe_literal_msg_some_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: "ls",
          env: env,
          pwd: loc,
          msg: Some(format!("!")),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_exe_expr_msg_some_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: Some(format!("!")),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_exe_literal_msg_none<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: "ls",
          env: env,
          pwd: loc,
          msg: None,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_exe_expr_msg_none<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: None,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_exe_literal_no_msg<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: "ls",
          env: env,
          pwd: loc,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_exe_expr_no_msg<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: format!("ls {}", "."),
          env: env,
          pwd: loc,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_no_trailing_comma<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! { exe: "ls", env: env, pwd: loc }
    }
}
