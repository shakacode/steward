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
//! ```rust
//! #[macro_use]
//! extern crate steward;
//!
//! use steward::{Cmd, Env, ProcesPool, Process};
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
//! ### Windows support
//! It wouldn't compile on Windows yet. No fundamental problems, I'm just not a user of this OS
//! and can't properly test the implementation. There is only one platform-specific place
//! in the crate that is related to a process termination. If you're a Windows user
//! and willing to help — PRs are totally welcome.
//!
//! ### Async runtimes
//! Tokio only.

#[macro_use]
extern crate lazy_static;

/// Base building block of the crate.
///
/// ```rust
/// let build_cmd = cmd! {
///     exe: "cargo build",
///     env: Env::empty(),
///     pwd: Loc::root(),
///     msg: "Building a server",
/// };
///
/// build_cmd.run().await
/// ```
#[macro_use]
pub mod cmd;
/// Long running process.
///
/// ```rust
///  let server_process = process! {
///      tag: "server",
///      cmd: cmd! {
///          exe: "cargo watch",
///          env: Env::empty(),
///          pwd: Loc::root(),
///          msg: "Running a reloadable server",
///      },
///  };
///
///  let client_process = process! {
///      tag: "client",
///      cmd: cmd! {
///          exe: "rescript build -w",
///          env: Env::empty(),
///          pwd: Loc::root(),
///          msg: "Watching a client",
///      },
///  };
///
/// ProcessPool::run(vec![server_process, client_process]).await
/// ```
#[macro_use]
pub mod process;
/// Command environment.
pub mod env;
/// [`Result`](Result) and [`Error`](Error) types of this crate.
pub mod result;

#[macro_use]
mod fmt;
mod loc;

pub use cmd::Cmd;
pub use env::Env;
pub use loc::Location;
pub use process::{Process, ProcessPool};
pub use result::{Error, Result};

pub(crate) use process::{ExitResult, RunningProcess};
