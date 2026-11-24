use std::os::unix::net::{UnixListener, UnixStream};
use tempfile::tempdir;
use yolo_local_lifecycle::service_pid;

#[tokio::test]
async fn service_pid_resolves_the_socket_owner() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("service.sock");
    let _listener = UnixListener::bind(&path).unwrap();
    let _client = UnixStream::connect(&path).unwrap();
    assert_eq!(service_pid(&path).await, Some(std::process::id()));
}

#[tokio::test]
async fn service_pid_returns_none_for_a_missing_socket() {
    let directory = tempdir().unwrap();
    assert_eq!(
        service_pid(&directory.path().join("absent.sock")).await,
        None
    );
}
