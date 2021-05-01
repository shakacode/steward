#[macro_use]
extern crate steward;
#[macro_use]
extern crate lazy_static;

mod config;
mod loc;

use config::Config;
use loc::Loc;

pub type Cmd = steward::Cmd<Loc>;
pub type Process = steward::Process<Loc>;
pub type ProcessPool = steward::ProcessPool;

#[tokio::main]
async fn main() -> steward::Result<()> {
    client::build().run().await?;
    server::build().run().await?;
    ProcessPool::run(vec![client::watch(), server::watch()]).await?;
    Ok(())
}

mod client {
    use steward::env::{Env, PATH};

    use crate::{Cmd, Loc, Process};

    fn env() -> Env {
        // Extending PATH with node_modules binaries
        Env::one("PATH", PATH::extend(Loc::client_node_modules_bin()))
    }

    pub fn build() -> Cmd {
        cmd! {
            exe: "rescript build -with-deps",
            env: env(),
            pwd: Loc::client(),
            msg: "Building ReScript client",
        }
    }

    pub fn watch() -> Process {
        process! {
            tag: "rescript",
            cmd:
                cmd! {
                    exe: "rescript build -w",
                    env: env(),
                    pwd: Loc::client(),
                    msg: "Watching ReScript client",
                }
        }
    }
}

mod server {
    use crate::{Cmd, Config, Loc, Process};

    pub fn build() -> Cmd {
        cmd! {
            exe: "cargo build --package=server",
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
                    exe: "cargo watch --watch server --exec 'run --package=server --color=always'",
                    env: Config::to_env(),
                    pwd: Loc::root(),
                    msg: "Running reloadable Rust server",
                }
        }
    }
}
