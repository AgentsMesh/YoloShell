use crate::worker::tests::*;
const REPLACEMENT_CONTENT: &[u8] = b"replacement";

#[tokio::test]
async fn worker_socket_cleanup_removes_only_the_captured_socket() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let path = paths.worker_socket(terminal_id).unwrap();
    let (listener, cleanup) = bind_worker_socket(&paths, terminal_id, &path).await.unwrap();
    assert!(worker_socket_identity(&path).is_ok());
    drop(listener);
    drop(cleanup);
    assert!(!path.exists());
    assert!(!paths.socket_dir().unwrap().exists());

    let namespace = paths.retain_socket_namespace().unwrap();
    let (listener, cleanup) = bind_worker_socket(&paths, terminal_id, &path).await.unwrap();
    drop(listener);
    std::fs::remove_file(&path).unwrap();
    std::fs::write(&path, REPLACEMENT_CONTENT).unwrap();
    drop(cleanup);
    assert_eq!(std::fs::read(&path).unwrap(), REPLACEMENT_CONTENT);
    assert!(worker_socket_identity(&path).is_err());
    std::fs::remove_file(&path).unwrap();
    drop(namespace);
    assert!(!paths.socket_dir().unwrap().exists());
}

#[tokio::test]
async fn worker_socket_binding_replaces_stale_but_rejects_live_endpoints() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let stale_session = Uuid::new_v4();
    paths.ensure_session(stale_session).unwrap();
    let _namespace = paths.retain_socket_namespace().unwrap();
    let stale_path = paths.worker_socket(stale_session).unwrap();
    let stale = std::os::unix::net::UnixListener::bind(&stale_path).unwrap();
    set_current_worker_socket_permissions(&stale_path).unwrap();
    drop(stale);
    let (listener, cleanup) = bind_worker_socket(&paths, stale_session, &stale_path)
        .await
        .unwrap();
    drop(listener);
    drop(cleanup);
    assert!(!stale_path.exists());

    let live_session = Uuid::new_v4();
    paths.ensure_session(live_session).unwrap();
    let live_path = paths.worker_socket(live_session).unwrap();
    let live = std::os::unix::net::UnixListener::bind(&live_path).unwrap();
    set_current_worker_socket_permissions(&live_path).unwrap();
    let error = bind_worker_socket(&paths, live_session, &live_path)
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains("already accepting"));
    drop(live);
}
