#[macro_use]
extern crate steward;

mod config;
mod loc;

use clap::Parser;

use config::Config;
use loc::Loc;

use steward::{HttpMethod, HttpService, PoolEntry};

pub type Cmd = steward::Cmd<Loc>;
pub type Process = steward::Process<Loc>;
pub type ProcessPool = steward::ProcessPool;

#[derive(Parser)]
#[clap(
    name = "steward-demo",
    version = "1.0",
    author = "Alex Fedoseev <alex@35mil.me>"
)]
pub struct Cli {
    #[arg(long, help = "Run a process pool with dependent porcesses")]
    with_deps: bool,
}

#[tokio::main]
async fn main() -> steward::Result<()> {
    match Cli::parse() {
        Cli { with_deps: false } => {
            server::build().run().await?;
            client::build().run().await?;

            ProcessPool::run(vec![server::watch(), client::watch()]).await?;
        }
        Cli { with_deps: true } => {
            use std::time::Duration;

            server::build().run().await?;
            client::build().run().await?;

            ProcessPool::run_with_deps(vec![
                PoolEntry::Process(server::watch()),
                PoolEntry::ProcessWithDep {
                    process: client::watch(),
                    dependency: Box::new(HttpService {
                        tag: "server".to_string(),
                        addr: format!(
                            "http://{host}:{port}",
                            host = Config::SERVER_HOST(),
                            port = Config::SERVER_PORT()
                        )
                        .parse()
                        .unwrap(),
                        method: HttpMethod::GET,
                        timeout: Duration::from_secs(30),
                    }),
                },
            ])
            .await?;
        }
    }
    Ok(())
}

mod client {
    use steward::env::{Env, PATH};

    use crate::{loc, Cmd, Process};

    fn env() -> Env {
        // Extending PATH with node_modules binaries
        Env::one("PATH", PATH::extend(loc::client_node_modules_bin()))
    }

    pub fn build() -> Cmd {
        cmd! {
            "rescript build -with-deps",
            env: env(),
            pwd: loc::client(),
            msg: "Building ReScript client",
        }
    }

    pub fn watch() -> Process {
        process! {
            tag: "rescript",
            cmd:
                cmd! {
                    "rescript build -w",
                    env: env(),
                    pwd: loc::client(),
                    msg: "Watching ReScript client",
                }
        }
    }
}

mod server {
    use crate::{Cmd, Config, Loc, Process};

    pub fn build() -> Cmd {
        cmd! {
            "cargo build --package=server",
            env: Config::to_env(),
            pwd: Loc::root(),
            msg: "Building Rust server",
        }
    }

    pub fn watch() -> Process {
        process! {
            tag: "server",
            cmd:
                cmd! {
                    "cargo watch --watch server --exec 'run --package=server --color=always'",
                    env: Config::to_env(),
                    pwd: Loc::root(),
                    msg: "Running reloadable Rust server",
                }
        }
    }
}
