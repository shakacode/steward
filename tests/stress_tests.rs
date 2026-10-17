mod common;

use common::Loc;
use std::time::Duration;
use steward::{Cmd, Env, KillTimeout, Process, SpawnOptions};

#[tokio::test]
async fn rapid_spawn_and_kill_cycle() {
    for _ in 0..20 {
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

        let process = Process::new("stress", cmd, KillTimeout::new(Duration::from_secs(5)));
        let opts = SpawnOptions {
            stdout: std::process::Stdio::null(),
            stderr: std::process::Stdio::null(),
            timeout: KillTimeout::new(Duration::from_secs(5)),
            ..Default::default()
        };

        let running = process.spawn(opts).await.unwrap();
        running.stop().await.unwrap();
    }
}

#[tokio::test]
async fn rapid_spawn_output_cycle() {
    for i in 0..50 {
        let cmd = Cmd {
            exe: format!("echo iteration_{}", i),
            env: Env::empty(),
            pwd: Loc::root(),
            msg: None,
        };

        let output = cmd.output().await.unwrap();
        let text = output.as_string().unwrap();
        assert!(text.contains(&format!("iteration_{}", i)));
    }
}

#[tokio::test]
async fn large_env_spawn() {
    let mut env = Env::empty();
    for i in 0..100 {
        env = env.insert(format!("STRESS_VAR_{}", i), format!("value_{}", i));
    }

    let cmd = Cmd {
        exe: "echo env_test".to_string(),
        env,
        pwd: Loc::root(),
        msg: None,
    };

    let output = cmd.output().await.unwrap();
    assert!(output.as_string().unwrap().contains("env_test"));
}

#[tokio::test]
async fn concurrent_output_capture() {
    use std::sync::Arc;
    use tokio::task;

    let results = Arc::new(std::sync::Mutex::new(Vec::new()));

    let mut handles = Vec::new();
    for i in 0..10 {
        let results = results.clone();
        let handle = task::spawn(async move {
            let cmd = Cmd {
                exe: format!("echo concurrent_{}", i),
                env: Env::empty(),
                pwd: Loc::root(),
                msg: None,
            };
            let output = cmd.output().await.unwrap();
            let text = output.as_string().unwrap();
            results.lock().unwrap().push(text);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 10);
    for i in 0..10 {
        let expected = format!("concurrent_{}", i);
        assert!(
            results.iter().any(|text| text.contains(&expected)),
            "Missing output for iteration {}",
            i
        );
    }
}

#[tokio::test]
async fn spawn_with_process_group_rapid() {
    for _ in 0..10 {
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

        let process = Process::new(
            "group_stress",
            cmd,
            KillTimeout::new(Duration::from_secs(5)),
        );
        let opts = SpawnOptions {
            stdout: std::process::Stdio::null(),
            stderr: std::process::Stdio::null(),
            timeout: KillTimeout::new(Duration::from_secs(5)),
            group: true,
        };

        let running = process.spawn(opts).await.unwrap();
        running.stop().await.unwrap();
    }
}
