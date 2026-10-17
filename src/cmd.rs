use std::{io, ops::Deref, process::Stdio, sync::LazyLock, time::Duration};

use tokio::process::Command;

use crate::{Env, Location, Result, RunningProcess};

/// Struct holds a specification of a command. Can be used for running one-off commands, long running processes etc.
#[derive(Clone, Debug)]
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
    ///
    /// ```
    /// # use steward::KillTimeout;
    /// # use std::time::Duration;
    /// let timeout = KillTimeout::new(Duration::from_secs(30));
    /// assert_eq!(timeout.duration(), Duration::from_secs(30));
    /// ```
    pub fn new(duration: Duration) -> Self {
        Self(duration)
    }

    /// Returns underlying [`Duration`](std::time::Duration).
    pub fn duration(&self) -> Duration {
        self.0
    }
}

static DEFAULT_KILL_TIMEOUT: LazyLock<Duration> = LazyLock::new(|| {
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
#[derive(Debug)]
pub struct SpawnOptions {
    /// Stdout stream.
    pub stdout: Stdio,
    /// Stderr stream.
    pub stderr: Stdio,
    /// Amount of time to wait before killing hanged process. See [`KillTimeout`](crate::KillTimeout).
    pub timeout: KillTimeout,
    /// Whether to create a new process group. When true, the spawned process becomes
    /// a process group leader, allowing all its children to be killed together.
    pub group: bool,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            stdout: Stdio::inherit(),
            stderr: Stdio::inherit(),
            timeout: KillTimeout::default(),
            group: false,
        }
    }
}

/// Enum returned from [`Cmd::output`](Cmd::output).
#[derive(Debug)]
pub struct Output(Vec<u8>);

impl Output {
    /// Returns bytes from stdout. Be aware that if child process was interrupted
    /// during the command execution (e.g. user pressed Ctrl + C), this function will terminate
    /// current process with zero exit code.
    pub fn bytes(self) -> Vec<u8> {
        self.0
    }

    /// Same as [`Output::bytes`](Output::bytes) but attempts to convert bytes to `String`.
    pub fn as_string(self) -> Result<String> {
        let bytes = self.bytes();
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
    /// Returns shell arguments for invoking a command string via `/bin/sh`.
    pub(crate) fn shelled(cmd: &str) -> [&str; 2] {
        ["-c", cmd]
    }

    #[cfg(windows)]
    /// Returns shell arguments for invoking a command string via `cmd`.
    pub(crate) fn shelled(cmd: &str) -> [&str; 2] {
        ["/c", cmd]
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

        Ok(Output(res.stdout))
    }

    /// A low-level method for spawning a process and getting a handle to it.
    pub fn spawn(&self, opts: SpawnOptions) -> io::Result<RunningProcess> {
        let cmd = self;

        let SpawnOptions {
            stdout,
            stderr,
            timeout,
            group,
        } = opts;

        let mut command = Command::new(Cmd::<Loc>::SHELL);
        command
            .args(Cmd::<Loc>::shelled(&cmd.exe))
            .envs(&cmd.env)
            .current_dir(cmd.pwd.as_path())
            .stdout(stdout)
            .stderr(stderr);

        if group {
            #[cfg(unix)]
            command.process_group(0);
            #[cfg(windows)]
            command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP);
        }

        let process = command.spawn()?;

        Ok(RunningProcess {
            process,
            timeout,
            group,
        })
    }
}

/// Convenience macro for creating a [`Cmd`](Cmd).
///
/// ## Examples
/// General command:
/// ```ignore
/// cmd! {
///   "rm -rf target",
///   env: Env::empty(),
///   pwd: Loc::root(),
///   msg: "Removing target dir",
/// }
/// ```
///
/// Dynamically constructed command:
/// ```ignore
/// cmd! {
///   format!("rm -rf {}", dir),
///   env: Env::empty(),
///   pwd: Loc::root(),
///   msg: format!("Removing {} dir", dir),
/// }
/// ```
///
/// Command without a message:
/// ```ignore
/// cmd! {
///   "ls",
///   env: Env::empty(),
///   pwd: Loc::root(),
/// }
/// ```
///
/// You can label a command with `exe:` if you want to:
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
        $exe:literal,
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
        $exe:literal,
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
        $exe:literal,
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
        $exe:literal,
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
        $exe:expr,
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
        $exe:expr,
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
        $exe:expr,
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
        $exe:expr,
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
        $exe:literal,
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
        $exe:expr,
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
    fn cmd_macro_unlabeled_exe_literal_msg_literal<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          "ls",
          env: env,
          pwd: loc,
          msg: "!",
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_literal_msg_literal<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: "ls",
          env: env,
          pwd: loc,
          msg: "!",
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_unlabeled_exe_expr_msg_literal<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: "!",
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_expr_msg_literal<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: "!",
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_unlabeled_exe_expr_msg_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: "!".to_string(),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_expr_msg_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: "!".to_string(),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_unlabeled_exe_literal_msg_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          "ls",
          env: env,
          pwd: loc,
          msg: "!".to_string(),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_literal_msg_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: "ls",
          env: env,
          pwd: loc,
          msg: "!".to_string(),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_unlabeled_exe_literal_msg_some_expr<Loc: Location>(
        env: Env,
        loc: Loc,
    ) -> Cmd<Loc> {
        cmd! {
          "ls",
          env: env,
          pwd: loc,
          msg: Some("!".to_string()),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_literal_msg_some_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: "ls",
          env: env,
          pwd: loc,
          msg: Some("!".to_string()),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_unlabeled_exe_expr_msg_some_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: Some("!".to_string()),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_expr_msg_some_expr<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: Some("!".to_string()),
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_unlabeled_exe_literal_msg_none<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          "ls",
          env: env,
          pwd: loc,
          msg: None,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_literal_msg_none<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: "ls",
          env: env,
          pwd: loc,
          msg: None,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_unlabeled_exe_expr_msg_none<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: None,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_expr_msg_none<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: format!("ls {}", "."),
          env: env,
          pwd: loc,
          msg: None,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_unlabeled_exe_literal_no_msg<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          "ls",
          env: env,
          pwd: loc,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_literal_no_msg<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: "ls",
          env: env,
          pwd: loc,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_unlabeled_exe_expr_no_msg<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          format!("ls {}", "."),
          env: env,
          pwd: loc,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_expr_no_msg<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! {
          exe: format!("ls {}", "."),
          env: env,
          pwd: loc,
        }
    }

    #[allow(dead_code)]
    fn cmd_macro_unlabeled_exe_no_trailing_comma<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! { "ls", env: env, pwd: loc }
    }

    #[allow(dead_code)]
    fn cmd_macro_labeled_exe_no_trailing_comma<Loc: Location>(env: Env, loc: Loc) -> Cmd<Loc> {
        cmd! { exe: "ls", env: env, pwd: loc }
    }
}

#[cfg(test)]
mod timeout_tests {
    use super::*;

    #[test]
    fn kill_timeout_default_respects_env() {
        let timeout = KillTimeout::default();
        let expected = match std::env::var("PROCESS_TIMEOUT") {
            Ok(val) => val
                .parse::<u64>()
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(10)),
            Err(_) => Duration::from_secs(10),
        };
        assert_eq!(timeout.duration(), expected);
    }

    #[test]
    fn kill_timeout_new() {
        let timeout = KillTimeout::new(Duration::from_secs(30));
        assert_eq!(timeout.duration(), Duration::from_secs(30));
    }

    #[test]
    fn kill_timeout_from_duration() {
        let timeout: KillTimeout = Duration::from_secs(5).into();
        assert_eq!(timeout.duration(), Duration::from_secs(5));
    }

    #[test]
    fn kill_timeout_deref_to_duration() {
        let timeout = KillTimeout::new(Duration::from_secs(15));
        let dur: &Duration = &timeout;
        assert_eq!(*dur, Duration::from_secs(15));
    }

    #[test]
    fn kill_timeout_clone() {
        let timeout = KillTimeout::new(Duration::from_secs(7));
        let cloned = timeout.clone();
        assert_eq!(cloned.duration(), Duration::from_secs(7));
    }
}

#[cfg(test)]
mod output_tests {
    use super::*;

    #[test]
    fn output_bytes() {
        let output = Output(b"hello".to_vec());
        assert_eq!(output.bytes(), b"hello");
    }

    #[test]
    fn output_as_string_valid_utf8() {
        let output = Output(b"hello world".to_vec());
        assert_eq!(output.as_string().unwrap(), "hello world");
    }

    #[test]
    fn output_as_string_invalid_utf8() {
        let output = Output(vec![0xff, 0xfe]);
        assert!(output.as_string().is_err());
    }
}
