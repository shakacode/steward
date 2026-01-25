#![deny(missing_docs)]

//! Task runner and process manager for Rust.
//!
//! If you're not happy managing your infrastructure with a pile of shell scripts,
//! this crate might be helpful. It provides base building blocks for defining
//! and running various kinds of tasks.
//! It's like [`foreman`](https://github.com/ddollar/foreman) but more low-level, with Rust API
//! and more flexibility & features.
//!
//! Works great with [`clap`](https://github.com/clap-rs/clap)!
//!
//! ## Examples
//!
//! Check out runnable examples on GitHub: [`steward/examples`](https://github.com/alexfedoseev/steward/tree/master/examples).
//!
//! ```ignore
//! #[macro_use]
//! extern crate steward;
//!
//! use steward::{Cmd, Env, ProcessPool, Process};
//!
//! #[tokio::main]
//! async fn main() -> steward::Result<()> {
//!     client::build_cmd().run().await?;
//!     server::build_cmd().run().await?;
//!
//!     ProcessPool::run(vec![client::watcher(), server::watcher()]).await?;
//!
//!     Ok(())
//! }
//!
//! mod server {
//!     fn build_cmd() -> Cmd {
//!         cmd! {
//!           "cargo build",
//!           env: Env::empty(),
//!           pwd: Loc::root(),
//!           msg: "Building a server",
//!         }
//!     }
//!
//!     fn watcher() -> Process {
//!         process! {
//!           tag: "server",
//!           cmd: cmd! {
//!             "cargo watch",
//!             env: Env::empty(),
//!             pwd: Loc::root(),
//!             msg: "Running a reloadable server",
//!           },
//!         }
//!     }
//! }
//!
//! mod client {
//!     fn build_cmd() -> Cmd {
//!         cmd! {
//!           "npm build",
//!           env: Env::empty(),
//!           pwd: Loc::root(),
//!           msg: "Building a client",
//!         }
//!     }
//!
//!     fn watcher() -> Process {
//!         process! {
//!           tag: "client",
//!           cmd: cmd! {
//!             "npm watch",
//!             env: Env::empty(),
//!             pwd: Loc::root(),
//!             msg: "Watching a client",
//!           },
//!         }
//!     }
//! }
//! ```
//!
//! ## Limitations
//! ### Windows
//! Apparently, Windows build is broken on recent versions of Rust due to
//! [`winapi`](https://github.com/retep998/winapi-rs) being unmaintained.
//! We need to migrate to [`windows-rs`](https://github.com/microsoft/windows-rs),
//! but I don't know anything about Windows, so help is very welcome!
//!
//! ### Async runtimes
//! Tokio only.

/// Base building block of the crate.
///
/// ```ignore
/// async fn build() -> steward::Result<()> {
///     let build_cmd = cmd! {
///         "cargo build",
///         env: Env::empty(),
///         pwd: Loc::root(),
///         msg: "Building a server",
///     };
///
///     build_cmd.run().await
/// }
/// ```
#[macro_use]
pub mod cmd;
/// Long running processes.
///
/// ```ignore
/// async fn run() -> steward::Result<()> {
///     let server_process = process! {
///         tag: "server",
///         cmd: cmd! {
///             "cargo watch",
///             env: Env::empty(),
///             pwd: Loc::root(),
///             msg: "Running a reloadable server",
///         },
///     };
///
///     let client_process = process! {
///         tag: "client",
///         cmd: cmd! {
///             "rescript build -w",
///             env: Env::empty(),
///             pwd: Loc::root(),
///             msg: "Watching a client",
///         },
///     };
///
///     ProcessPool::run(vec![server_process, client_process]).await
/// }
/// ```
#[macro_use]
pub mod process;
/// Dependant processes.
///
/// Sometimes, a job or a service depends on something else to function properly. For example, to generate a GraphQL
/// schema, a server must be available. Or to start a server, DB must be up and running. To handle
/// such cases, there is a [`Dependency`](crate::Dependency) trait.
///
/// Types that implement this trait are able to check if a dependency is available or to wait until
/// the dependency becomes available. For example, if some job requires a TCP service for some job:
///
/// ```ignore
/// let service = TcpService::new(...);
///
/// service.wait().await.unwrap();
///
/// // here, the service is available...
/// job.run().await;
/// ```
///
/// You can use provided [`TcpService`](crate::TcpService), [`HttpService`](crate::HttpService),
/// and [`FsEntry`](crate::FsEntry). Or implement your own
/// (you would need [`async_trait`](https://docs.rs/async-trait/latest/async_trait/)).
///
/// ## Process pool
///
/// It is also useful when you need to spawn a pool of long-running processes,
/// and some of them depend on something else to start properly,
/// such as an HTTP service being available or a file existing.
///
/// ```ignore
///  async fn run() -> steward::Result<()> {
///      server::build().run().await?;
///      client::build().run().await?;
///
///      ProcessPool::run_with_deps(vec![
///          PoolEntry::Process(server::watch()),
///          PoolEntry::ProcessWithDep {
///              process: client::watch(),
///              dependency: Box::new(HttpService {
///                  tag: "server".to_string(),
///                  addr: format!(
///                      "http://{host}:{port}",
///                      host = Config::SERVER_HOST(),
///                      port = Config::SERVER_PORT()
///                  )
///                  .parse()
///                  .unwrap(),
///                  method: HttpMethod::GET,
///                  timeout: Duration::from_secs(30),
///              }),
///          },
///      ])
///      .await
/// }
/// ```
pub mod dep;
/// Command environment.
pub mod env;
/// File system related types.
pub mod fs;
/// Network related types.
pub mod net;
/// [`Result`](Result) and [`Error`](Error) types of this crate.
pub mod result;

#[macro_use]
mod fmt;
mod fun;
mod loc;

pub use cmd::{Cmd, KillTimeout, SpawnOptions};
pub use dep::{Dependency, DependencyWaitError};
pub use env::Env;
pub use fmt::print;
pub use fs::FsEntry;
pub use fun::{run, run_mut, run_once};
pub use loc::Location;
pub use net::{HttpMethod, HttpService, TcpService};
pub use process::{PoolEntry, Process, ProcessPool, RunningProcess};
pub use result::{Error, Result};
