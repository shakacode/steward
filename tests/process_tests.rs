mod common;

use common::Loc;
use std::time::Duration;
use steward::{Cmd, Env, KillTimeout, Process, SpawnOptions};

#[tokio::test]
async fn process_spawn_and_stop() {
    let cmd = Cmd {
        exe: if cfg!(windows) {
            "ping -n 30 127.0.0.1".to_string()
        } else {
            "sleep 30".to_string()
        },
        env: Env::empty(),
        pwd: Loc::root(),
        msg: None,
    };

    let process = Process::new("test", cmd, KillTimeout::new(Duration::from_secs(5)));
    let opts = SpawnOptions {
        stdout: std::process::Stdio::piped(),
        stderr: std::process::Stdio::piped(),
        timeout: KillTimeout::new(Duration::from_secs(5)),
        ..Default::default()
    };

    let running = process.spawn(opts).await.unwrap();
    assert!(running.as_child().id().is_some());

    running.stop().await.unwrap();
}

#[tokio::test]
async fn process_spawn_with_group() {
    let cmd = Cmd {
        exe: if cfg!(windows) {
            "ping -n 30 127.0.0.1".to_string()
        } else {
            "sleep 30".to_string()
        },
        env: Env::empty(),
        pwd: Loc::root(),
        msg: None,
    };

    let process = Process::new("test", cmd, KillTimeout::new(Duration::from_secs(5)));
    let opts = SpawnOptions {
        stdout: std::process::Stdio::piped(),
        stderr: std::process::Stdio::piped(),
        timeout: KillTimeout::new(Duration::from_secs(5)),
        group: true,
    };

    let running = process.spawn(opts).await.unwrap();
    running.stop().await.unwrap();
}

#[tokio::test]
async fn process_macro_construction() {
    use steward::{cmd, process};

    let cmd = cmd! {
        "echo hello",
        env: Env::empty(),
        pwd: Loc::root(),
    };

    let process = process! {
        tag: "test",
        cmd: cmd,
        timeout: Duration::from_secs(20).into(),
    };

    assert_eq!(process.tag(), "test");
    assert_eq!(process.timeout().duration(), Duration::from_secs(20));
}

#[tokio::test]
async fn process_macro_default_timeout() {
    use steward::{cmd, process};

    let cmd = cmd! {
        "echo hello",
        env: Env::empty(),
        pwd: Loc::root(),
    };

    let process = process! {
        tag: "test",
        cmd: cmd,
    };

    assert_eq!(process.tag(), "test");
}

#[tokio::test]
async fn running_process_into_child() {
    let cmd = Cmd {
        exe: if cfg!(windows) {
            "ping -n 5 127.0.0.1".to_string()
        } else {
            "sleep 5".to_string()
        },
        env: Env::empty(),
        pwd: Loc::root(),
        msg: None,
    };

    let process = Process::new("test", cmd, KillTimeout::new(Duration::from_secs(5)));
    let opts = SpawnOptions {
        stdout: std::process::Stdio::piped(),
        stderr: std::process::Stdio::piped(),
        timeout: KillTimeout::new(Duration::from_secs(5)),
        ..Default::default()
    };

    let running = process.spawn(opts).await.unwrap();
    let mut child = running.into_child();
    child.kill().await.unwrap();
}
