mod common;

use common::Loc;
use std::time::Duration;
use steward::{Cmd, Env, KillTimeout, Location, PoolEntry, Process, ProcessPool};

fn make_quick_process(tag: &'static str, cmd_str: &str) -> Process<Loc> {
    Process::new(
        tag,
        Cmd {
            exe: cmd_str.to_string(),
            env: Env::empty(),
            pwd: Loc::root(),
            msg: None,
        },
        KillTimeout::new(Duration::from_secs(5)),
    )
}

#[test]
fn pool_construction_empty() {
    let pool: Vec<Process<Loc>> = vec![];
    assert_eq!(pool.len(), 0);
}

#[test]
fn pool_construction_single_process() {
    let pool = [make_quick_process("test", "echo hello")];
    assert_eq!(pool.len(), 1);
    assert_eq!(pool[0].tag(), "test");
}

#[test]
fn pool_construction_multiple_processes() {
    let pool = [
        make_quick_process("web", "echo web"),
        make_quick_process("api", "echo api"),
        make_quick_process("worker", "echo worker"),
    ];
    assert_eq!(pool.len(), 3);
    assert_eq!(pool[0].tag(), "web");
    assert_eq!(pool[1].tag(), "api");
    assert_eq!(pool[2].tag(), "worker");
}

#[test]
fn pool_entry_process_variant() {
    let process = make_quick_process("test", "echo hello");
    let entry: PoolEntry<Loc, dyn steward::Dependency> = PoolEntry::Process(process);
    match entry {
        PoolEntry::Process(p) => assert_eq!(p.tag(), "test"),
        _ => panic!("Expected Process variant"),
    }
}

#[test]
fn pool_entry_process_with_dep_variant() {
    use steward::FsEntry;

    let process = make_quick_process("test", "echo hello");
    let dep = FsEntry {
        tag: "cargo_toml".to_string(),
        addr: Loc::root().join("Cargo.toml"),
        timeout: Duration::from_secs(5),
    };

    let entry: PoolEntry<Loc, dyn steward::Dependency> = PoolEntry::ProcessWithDep {
        process,
        dependency: Box::new(dep),
    };

    match entry {
        PoolEntry::ProcessWithDep {
            process,
            dependency,
        } => {
            assert_eq!(process.tag(), "test");
            assert_eq!(dependency.tag(), "cargo_toml");
        }
        _ => panic!("Expected ProcessWithDep variant"),
    }
}

#[tokio::test]
async fn pool_processes_have_correct_timeouts() {
    let p1 = Process::new(
        "fast",
        Cmd {
            exe: "echo fast".to_string(),
            env: Env::empty(),
            pwd: Loc::root(),
            msg: None,
        },
        KillTimeout::new(Duration::from_secs(3)),
    );

    let p2 = Process::new(
        "slow",
        Cmd {
            exe: "echo slow".to_string(),
            env: Env::empty(),
            pwd: Loc::root(),
            msg: None,
        },
        KillTimeout::new(Duration::from_secs(30)),
    );

    assert_eq!(p1.timeout().duration(), Duration::from_secs(3));
    assert_eq!(p2.timeout().duration(), Duration::from_secs(30));
}

#[tokio::test]
async fn pool_process_bad_command_exits_with_error() {
    let bad_process = Process::new(
        "bad",
        Cmd {
            exe: "nonexistent_command_xyz_123".to_string(),
            env: Env::empty(),
            pwd: Loc::root(),
            msg: None,
        },
        KillTimeout::new(Duration::from_secs(5)),
    );

    let opts = steward::SpawnOptions {
        stdout: std::process::Stdio::null(),
        stderr: std::process::Stdio::null(),
        ..Default::default()
    };

    let running = bad_process.spawn(opts).await;
    assert!(
        running.is_ok(),
        "Shell spawn succeeds even for bad commands"
    );

    let result = running.unwrap().into_child().wait().await.unwrap();
    assert!(
        !result.success(),
        "Bad command should exit with non-zero code"
    );
}

#[tokio::test]
async fn pool_with_deps_construction() {
    use steward::FsEntry;

    let server = make_quick_process("server", "echo server");
    let client = make_quick_process("client", "echo client");

    let dep = FsEntry {
        tag: "config".to_string(),
        addr: Loc::root().join("Cargo.toml"),
        timeout: Duration::from_secs(5),
    };

    let pool: Vec<PoolEntry<Loc, dyn steward::Dependency>> = vec![
        PoolEntry::Process(server),
        PoolEntry::ProcessWithDep {
            process: client,
            dependency: Box::new(dep),
        },
    ];

    assert_eq!(pool.len(), 2);
}

#[test]
fn process_pool_debug() {
    let debug_str = format!("{:?}", ProcessPool);
    assert_eq!(debug_str, "ProcessPool");
}
