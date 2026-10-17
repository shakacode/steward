mod common;

use common::Loc;
use steward::{Cmd, Env};

#[tokio::test]
async fn cmd_run_succeeds() {
    let cmd = Cmd {
        exe: "echo hello".to_string(),
        env: Env::empty(),
        pwd: Loc::root(),
        msg: None,
    };
    cmd.run().await.unwrap();
}

#[tokio::test]
async fn cmd_run_fails_on_bad_command() {
    let cmd = Cmd {
        exe: "nonexistent_command_xyz_123".to_string(),
        env: Env::empty(),
        pwd: Loc::root(),
        msg: None,
    };
    let result = cmd.run().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn cmd_silent_succeeds() {
    let cmd = Cmd {
        exe: "echo hello".to_string(),
        env: Env::empty(),
        pwd: Loc::root(),
        msg: None,
    };
    cmd.silent().await.unwrap();
}

#[tokio::test]
async fn cmd_output_captures_stdout() {
    let cmd = Cmd {
        exe: "echo steward_test_output".to_string(),
        env: Env::empty(),
        pwd: Loc::root(),
        msg: None,
    };
    let output = cmd.output().await.unwrap();
    let text = output.as_string().unwrap();
    assert!(text.contains("steward_test_output"));
}

#[tokio::test]
async fn cmd_output_returns_error_on_nonzero_exit() {
    let cmd = Cmd {
        exe: "exit 1".to_string(),
        env: Env::empty(),
        pwd: Loc::root(),
        msg: None,
    };
    let result = cmd.output().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn cmd_spawn_and_wait() {
    let cmd = Cmd {
        exe: "echo spawned".to_string(),
        env: Env::empty(),
        pwd: Loc::root(),
        msg: None,
    };
    let opts = steward::SpawnOptions {
        stdout: std::process::Stdio::piped(),
        stderr: std::process::Stdio::piped(),
        ..Default::default()
    };
    let process = cmd.spawn(opts).unwrap();
    let result = process.as_child();
    assert!(result.id().is_some());
}

#[tokio::test]
async fn cmd_with_env() {
    let cmd = Cmd {
        exe: if cfg!(windows) {
            "echo %STEWARD_TEST%".to_string()
        } else {
            "echo $STEWARD_TEST".to_string()
        },
        env: Env::one("STEWARD_TEST", "env_works"),
        pwd: Loc::root(),
        msg: None,
    };
    let output = cmd.output().await.unwrap();
    let text = output.as_string().unwrap();
    assert!(text.contains("env_works"));
}

#[tokio::test]
async fn cmd_macro_construction() {
    use steward::cmd;

    let cmd = cmd! {
        "echo macro_test",
        env: Env::empty(),
        pwd: Loc::root(),
        msg: "Testing macro",
    };
    assert_eq!(cmd.exe(), "echo macro_test");
    assert_eq!(cmd.msg().unwrap(), "Testing macro");
}
