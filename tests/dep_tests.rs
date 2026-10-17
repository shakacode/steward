mod common;

use common::Loc;
use std::time::Duration;
use steward::{Dependency, FsEntry, Location};

#[tokio::test]
async fn fs_entry_check_existing_file() {
    let fs = FsEntry {
        tag: "test".to_string(),
        addr: Loc::root().join("Cargo.toml"),
        timeout: Duration::from_secs(1),
    };
    assert!(fs.check().await.is_ok());
}

#[tokio::test]
async fn fs_entry_check_nonexistent_file() {
    let fs = FsEntry {
        tag: "test".to_string(),
        addr: Loc::root().join("nonexistent_file_xyz_123"),
        timeout: Duration::from_secs(1),
    };
    assert!(fs.check().await.is_err());
}

#[tokio::test]
async fn fs_entry_wait_existing_file_returns_immediately() {
    let fs = FsEntry {
        tag: "test".to_string(),
        addr: Loc::root().join("Cargo.toml"),
        timeout: Duration::from_secs(5),
    };
    let start = std::time::Instant::now();
    fs.wait().await.unwrap();
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn fs_entry_wait_nonexistent_times_out() {
    let fs = FsEntry {
        tag: "test".to_string(),
        addr: Loc::root().join("nonexistent_file_xyz_123"),
        timeout: Duration::from_millis(500),
    };
    let result = fs.wait().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn fs_entry_wait_creates_file_during_wait() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("steward_test_{}", std::process::id()));

    let fs = FsEntry {
        tag: "test".to_string(),
        addr: Loc(temp_dir.clone()).join(format!("steward_test_{}", std::process::id())),
        timeout: Duration::from_secs(5),
    };

    let fs_clone_path = file_path.clone();
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        std::fs::write(&fs_clone_path, "test").unwrap();
    });

    fs.wait().await.unwrap();
    handle.await.unwrap();

    let _ = std::fs::remove_file(&file_path);
}

#[tokio::test]
async fn tcp_service_check_unreachable_port() {
    use steward::TcpService;

    let service = TcpService::new(
        "test",
        "127.0.0.1",
        "19999",
        Duration::from_millis(200),
        None,
    )
    .unwrap();
    assert!(service.check().await.is_err());
}

#[tokio::test]
async fn tcp_service_wait_unreachable_times_out() {
    use steward::TcpService;

    let service = TcpService::new(
        "test",
        "127.0.0.1",
        "19999",
        Duration::from_millis(500),
        None,
    )
    .unwrap();
    let result = service.wait().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tcp_service_tag() {
    use steward::TcpService;

    let service = TcpService::new(
        "my_service",
        "127.0.0.1",
        "8080",
        Duration::from_secs(1),
        None,
    )
    .unwrap();
    assert_eq!(service.tag(), "my_service");
}

#[tokio::test]
async fn http_service_tag() {
    use steward::{HttpMethod, HttpService};

    let service = HttpService::new(
        "my_http",
        "127.0.0.1",
        "8080",
        "/health",
        false,
        HttpMethod::GET,
        Duration::from_secs(1),
    )
    .unwrap();
    assert_eq!(service.tag(), "my_http");
}
