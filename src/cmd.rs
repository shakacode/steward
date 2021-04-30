use std::{
    process::{self, Stdio},
    time::Duration,
};

use crate::{process::TIMEOUT, Env, ExitResult, Location, Result, RunningProcess};

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
    Loc: Location + Send + Sync,
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

    /// Runs one-off command with inherited [`Stdio`](std::process::Stdio). Prints headline (witn [`Cmd::msg`](Cmd::msg), if provided) to stderr.
    pub async fn run(&self) -> Result<()> {
        eprintln!("{}", crate::headline!(self));
        self.spawn(Stdio::inherit(), Stdio::inherit()).await?;
        Ok(())
    }

    /// Runs one-off command. Doesn't print anything.
    pub async fn silent(&self) -> Result<()> {
        self.spawn(Stdio::null(), Stdio::null()).await?;
        Ok(())
    }

    /// Runs one-off command and returns [`Output`](Output). Doesn't print anything.
    pub async fn output(&self) -> Result<Output> {
        let res = self.spawn(Stdio::piped(), Stdio::piped()).await?;
        match res {
            ExitResult::Output(output) => Ok(Output::Data(output.stdout)),
            ExitResult::Interrupted | ExitResult::Killed { pid: _ } => Ok(Output::Interrupted),
        }
    }

    async fn spawn(&self, stdout: Stdio, stderr: Stdio) -> Result<ExitResult> {
        let cmd = self;
        RunningProcess::spawn(cmd, stdout, stderr, Duration::from_secs(*TIMEOUT))
            .await?
            .wait()
            .await
    }

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
}

/// Convenience macro for creating a [`Cmd`](Cmd).
///
/// ## Examples
/// General command:
/// ```rust
/// cmd! {
///   exe: "rm -rf target",
///   env: Env::empty(),
///   pwd: Loc::root(),
///   msg: "Removing target dir",
/// }
/// ```
///
/// Dynamically constructed command:
/// ```rust
/// cmd! {
///   exe: format!("rm -rf {}", dir),
///   env: Env::empty(),
///   pwd: Loc::root(),
///   msg: format!("Removing {} dir", dir),
/// }
/// ```
///
/// Command without a message:
/// ```rust
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
