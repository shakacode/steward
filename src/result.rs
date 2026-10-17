use std::{io, process, string};

/// Result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Error raised when a process manager failed to kill hanged process after timeout. It is platform-specific.
#[cfg(unix)]
pub type KillError = nix::Error;

/// Error raised when a process manager failed to kill hanged process after timeout. It is platform-specific.
#[cfg(windows)]
pub type KillError = u32;

/// Error type of this crate.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// IO error that might happen during command / process execution.
    #[error("IO error: {0}")]
    IoError(io::Error),
    /// Process was interrupted by user (`Ctrl + C`).
    #[error("Interrupted.")]
    Interrupted,
    /// Process was killed because it couldn't exit gracefully.
    #[error("Killed [pid: {pid}].")]
    Killed {
        /// Process identifier.
        pid: u32,
    },
    /// Error raised when a process exits with a non-zero exit code.
    #[error("Process exited with non-zero code: {code:?}. Output: {output:#?}")]
    NonZeroExitCode {
        /// Exit code of a process. Might be absent on Unix systems when a process was terminated by a signal.
        code: Option<i32>,
        /// [`Output`](std::process::Output) of the exited process
        output: process::Output,
    },
    /// Error raised when a child process does not return its identifier,
    /// which means it does not exist at operating system level,
    /// which is unexpected in the context of this program.
    #[error("Process does not exist.")]
    ProcessDoesNotExist,
    /// When a process manager failed to kill hanged child process, there is a zombie process left hanging around.
    /// This error provides details, such as process id and an error, so user could handle cleaning manually.
    #[error("Process with pid {pid} hanged and we were unable to kill it. Error: {err}")]
    Zombie {
        /// Process id of the hanged process.
        pid: u32,
        /// Error raised on attempt to terminate the hanged process.
        err: KillError,
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
        debug_assert!(
            !output.status.success(),
            "Attempted to convert a successful command output into an error"
        );
        Self::NonZeroExitCode {
            code: output.status.code(),
            output,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_io_error_display() {
        let err = Error::IoError(io::Error::new(io::ErrorKind::NotFound, "file missing"));
        assert_eq!(err.to_string(), "IO error: file missing");
    }

    #[test]
    fn error_interrupted_display() {
        let err = Error::Interrupted;
        assert_eq!(err.to_string(), "Interrupted.");
    }

    #[test]
    fn error_killed_display() {
        let err = Error::Killed { pid: 1234 };
        assert_eq!(err.to_string(), "Killed [pid: 1234].");
    }

    #[test]
    fn error_process_does_not_exist_display() {
        let err = Error::ProcessDoesNotExist;
        assert_eq!(err.to_string(), "Process does not exist.");
    }

    #[test]
    fn error_zombie_display() {
        #[cfg(unix)]
        let err = Error::Zombie {
            pid: 42,
            err: nix::errno::Errno::EPERM,
        };
        #[cfg(windows)]
        let err = Error::Zombie { pid: 42, err: 5 };
        let msg = err.to_string();
        assert!(msg.contains("42"));
    }

    #[test]
    fn from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "no access");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::IoError(_)));
        assert_eq!(err.to_string(), "IO error: no access");
    }

    #[test]
    fn from_utf8_error() {
        let bad_bytes = vec![0xff, 0xfe];
        let utf8_err = String::from_utf8(bad_bytes).unwrap_err();
        let err: Error = utf8_err.into();
        assert!(matches!(err, Error::IoError(_)));
    }

    #[test]
    fn from_process_output_nonzero_exit() {
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;
        use std::process::{ExitStatus, Output};

        #[cfg(unix)]
        let status = ExitStatus::from_raw(256);
        #[cfg(windows)]
        let status = {
            use std::os::windows::process::ExitStatusExt;
            ExitStatus::from_raw(1)
        };

        let output = Output {
            status,
            stdout: b"out".to_vec(),
            stderr: b"err".to_vec(),
        };
        let err: Error = output.into();
        assert!(matches!(err, Error::NonZeroExitCode { .. }));
    }
}
