#![deny(missing_docs)]

//! Task runner and process manager for Rust.
//!
//! If you're not happy managing your infrastructure with a pile of bash scripts,
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
//!           exe: "cargo build",
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
//!             exe: "cargo watch",
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
//!           exe: "npm build",
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
//!             exe: "npm watch",
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
//! ### Async runtimes
//! Tokio only.

#[macro_use]
extern crate lazy_static;

/// Base building block of the crate.
///
/// ```ignore
/// async fn build() -> steward::Result<()> {
///     let build_cmd = cmd! {
///         exe: "cargo build",
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
/// Long running process.
///
/// ```ignore
/// async fn run() -> steward::Result<()> {
///     let server_process = process! {
///         tag: "server",
///         cmd: cmd! {
///             exe: "cargo watch",
///             env: Env::empty(),
///             pwd: Loc::root(),
///             msg: "Running a reloadable server",
///         },
///     };
///
///     let client_process = process! {
///         tag: "client",
///         cmd: cmd! {
///             exe: "rescript build -w",
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
/// Useful when you need to spawn a number of long-running processes,
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
///              dependency: Box::new(HttpDep {
///                  tag: "server".to_string(),
///                  host: Config::SERVER_HOST().to_owned(),
///                  port: Config::SERVER_PORT().to_owned(),
///                  path: "/".to_string(),
///                  timeout: Some(Duration::from_secs(30)),
///                  ..Default::default()
///              }),
///          },
///      ])
///      .await
/// }
/// ```
pub mod dep;
/// Command environment.
pub mod env;
/// [`Result`](Result) and [`Error`](Error) types of this crate.
pub mod result;

#[macro_use]
mod fmt;
mod loc;

pub use cmd::Cmd;
pub use dep::{Dependency, FsDep, HttpDep, HttpMethod, TcpDep};
pub use env::Env;
pub use loc::Location;
pub use process::{PoolEntry, Process, ProcessPool};
pub use result::{Error, Result};

pub(crate) use process::{ExitResult, RunningProcess};
