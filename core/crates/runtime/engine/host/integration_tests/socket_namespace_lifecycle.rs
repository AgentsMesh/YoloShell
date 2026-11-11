use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn host_publication_releases_before_the_last_namespace_guard() {
    let root = tempdir().unwrap();
    let host = Host::open(HostConfig::new(root.path().join("home"))).unwrap();
    let directory = host.paths.socket_dir().unwrap();
    let (listener, publication) = publish_test_socket(&host.socket_namespace, &host.socket).await;
    let cleanup = SocketCleanup::capture(publication, Arc::clone(&host.socket_namespace));
    drop(listener);
    drop(host);
    assert!(directory.is_dir());
    drop(cleanup);
    assert!(!directory.exists());
}

#[tokio::test]
async fn serving_guard_keeps_publication_until_the_last_host_owner_exits() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let host = Host::open(HostConfig::new(&home)).unwrap();
    let directory = host.paths.socket_dir().unwrap();
    let socket = host.socket.clone();
    let (listener, publication) = publish_test_socket(&host.socket_namespace, &socket).await;
    let mut serving = ServingHost::capture(host, publication);
    let retained_host = Arc::clone(serving.host());
    drop(listener);

    serving.release();
    assert!(socket.exists());
    assert!(Host::open(HostConfig::new(&home)).is_err());

    drop(retained_host);
    assert!(!socket.exists());
    let replacement = Host::open(HostConfig::new(&home)).unwrap();
    drop(replacement);
    assert!(!directory.exists());
}

#[tokio::test]
async fn post_publish_namespace_failure_keeps_locks_until_publication_cleanup() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let displaced = root.path().join("displaced-home");
    let host = Host::open(HostConfig::new(&home)).unwrap();
    let socket = host.socket.clone();
    let (listener, publication) = publish_test_socket(&host.socket_namespace, &socket).await;
    let serving = ServingHost::capture(host, publication);
    std::fs::rename(&home, &displaced).unwrap();
    Paths::new(&home).ensure_base().unwrap();

    assert!(serving.host().verify_namespace().is_err());
    assert!(Host::open(HostConfig::new(&home)).is_err());
    drop(listener);
    drop(serving);

    assert!(!socket.exists());
    drop(Host::open(HostConfig::new(&home)).unwrap_or_else(|error| {
        let entries = std::fs::read_dir(socket.parent().unwrap()).map(|entries| {
            entries
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<Vec<_>>()
        });
        panic!("replacement Host admission failed: {error:#}; transport entries: {entries:?}");
    }));
}

#[tokio::test]
async fn instance_locks_cover_publication_inspection_and_unlink() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let host = Host::open(HostConfig::new(&home)).unwrap();
    let socket = host.socket.clone();
    let (listener, mut publication) = publish_test_socket(&host.socket_namespace, &socket).await;
    let replacement_was_blocked = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let hook_result = Arc::clone(&replacement_was_blocked);
    crate::paths::test_support::set_socket_before_unlink(&mut publication, move || {
        hook_result.store(
            Host::open(HostConfig::new(&home)).is_err(),
            std::sync::atomic::Ordering::SeqCst,
        );
    });
    let serving = ServingHost::capture(host, publication);
    drop(listener);

    drop(serving);

    assert!(replacement_was_blocked.load(std::sync::atomic::Ordering::SeqCst));
    assert!(!socket.exists());
}

#[tokio::test]
async fn worker_guard_keeps_namespace_until_its_socket_is_released() {
    let root = tempdir().unwrap();
    let host = Host::open(HostConfig::new(root.path().join("home"))).unwrap();
    let directory = host.paths.socket_dir().unwrap();
    let worker = host.paths.retain_socket_namespace().unwrap();
    let worker_path = host.paths.worker_socket(Uuid::new_v4()).unwrap();
    let (host_listener, host_publication) =
        publish_test_socket(&host.socket_namespace, &host.socket).await;
    let host_cleanup = SocketCleanup::capture(host_publication, Arc::clone(&host.socket_namespace));
    let (worker_listener, worker_publication) = publish_test_socket(&worker, &worker_path).await;
    drop(host_listener);
    drop(host);
    drop(host_cleanup);
    assert!(directory.is_dir());
    assert!(worker_path.exists());
    drop(worker_listener);
    drop(worker_publication);
    drop(worker);
    assert!(!directory.exists());
}
