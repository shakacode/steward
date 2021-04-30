use std::{io, process, string};

/// Result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Error raised when a process manager failed to kill hanged process after timeout. It is platform-specific.
#[cfg(unix)]
pub type KillError = nix::Error;

/// Error type of this crate.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// IO error that might happen during command / process execution.
    #[error("IO error: {0}")]
    IoError(io::Error),
    /// Error raised when a process exits with a non-zero exit code.
    #[error("Process exited with non-zero code: {:#?}. Output: {:#?}", .code, .output)]
    NonZeroExitCode {
        /// Exit code of a process. Might be absent on Unix systems when a process was terminated by a signal.
        code: Option<i32>,
        /// [`Output`](std::process::Output) of the exited process
        output: process::Output,
    },
    /// When a process manager failed to kill hanged child process, there is a zombie process left hanging around.
    /// This error provides details, such as process id and an error, so user could handle cleaning manually.
    #[cfg(unix)]
    #[error("Process with pid {pid} hanged and we were unable to kill it. Error: {err}", pid = .pid, err = .err)]
    Zombie {
        /// Process id of the hanged process.
        pid: u32,
        /// Error raised on attempt to terminate the hanged process.
        err: KillError,
    },
    /// When a process manager failed to kill hanged child process, there is a zombie process left hanging around.
    /// This error provides details, such as process id and an error, so user could handle cleaning manually.
    #[cfg(windows)]
    #[error("Process with pid {pid} hanged and we were unable to kill it. Error: {err}", pid = .pid, err = .err)]
    Zombie {
        /// Process id of the hanged process.
        pid: u32,
        /// Error raised on attempt to terminate the hanged process.
        err: winapi::shared::minwindef::DWORD,
    },
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<string::FromUtf8Error> for Error {
    fn from(err: string::FromUtf8Error) -> Self {
        Self::IoError(io::Error::new(io::ErrorKind::InvalidInput, err))
    }
}

impl From<process::Output> for Error {
    fn from(output: process::Output) -> Self {
        if output.status.success() {
            panic!("Failed to convert command output to error because the command succeeded. Output: {:#?}", output);
        }
        Self::NonZeroExitCode {
            code: output.status.code(),
            output,
        }
    }
}
