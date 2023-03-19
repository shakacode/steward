use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::time;

use crate::{Dependency, DependencyWaitError, Location};

/// File system entry.
pub struct FsEntry<Loc> {
    /// A tag used as an identificator of the FS entry in the output.
    pub tag: String,
    /// A location of the FS entry.
    pub addr: Loc,
    /// FS entry timeout.
    pub timeout: Duration,
}

#[derive(thiserror::Error, Debug)]
enum FsEntryWaitError {
    #[error("Timeout")]
    Timeout,
}

impl DependencyWaitError for FsEntryWaitError {}

const ITER_GAP: Duration = Duration::from_millis(250);

#[async_trait]
impl<Loc> Dependency for FsEntry<Loc>
where
    Loc: Location,
{
    fn tag(&self) -> &str {
        &self.tag
    }

    async fn check(&self) -> Result<(), ()> {
        let path = self.addr.as_path();

        if path.exists() {
            Ok(())
        } else {
            Err(())
        }
    }

    async fn wait(&self) -> Result<(), Box<dyn DependencyWaitError>> {
        let path = self.addr.as_path();

        let expiration = Instant::now() + self.timeout;

        loop {
            if path.exists() {
                break;
            } else {
                if Instant::now() > expiration {
                    return Err(Box::new(FsEntryWaitError::Timeout));
                }

                time::sleep(ITER_GAP).await
            }
        }

        Ok(())
    }
}
