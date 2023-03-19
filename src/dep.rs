use std::error::Error as StdError;

use async_trait::async_trait;

/// Dependency trait.
///
/// See [`dep`](crate::dep) module documentation.
#[async_trait]
pub trait Dependency: Send + Sync {
    /// A tag used as an identificator in output when process runs as a part of a [`ProcessPool`](crate::ProcessPool).
    fn tag(&self) -> &str;
    /// A method that checks if a dependency is available.
    async fn check(&self) -> Result<(), ()>;
    /// A method that resolves when a dependency becomes available.
    async fn wait(&self) -> Result<(), Box<dyn DependencyWaitError>>;
}

/// Error returned from the [`Dependency::wait`](Dependency::wait) method must implement this trait.
///
/// ```ignore
/// impl DependencyWaitError for MyDependencyWaitError {}
/// ```
pub trait DependencyWaitError: StdError + Send + Sync {}
