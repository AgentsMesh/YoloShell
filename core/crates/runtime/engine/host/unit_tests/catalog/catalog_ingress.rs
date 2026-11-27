use crate::host::tests::*;
use super::*;

const CATALOG_OPERATION_COUNT: usize = 2;
const CONTROLLER_CLAIM_TEST_TIMEOUT: Duration = Duration::from_secs(60);
const CONTROLLER_CLAIM_TIMEOUT: &str = "next host did not report its controller claim";
const NEW_GENERATION_SESSION_NAME: &str = "new-generation-session";

struct TestReleaseGuard(Option<std::sync::mpsc::Sender<()>>);

impl TestReleaseGuard {
    fn new(sender: std::sync::mpsc::Sender<()>) -> Self {
        Self(Some(sender))
    }

    fn release(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

impl Drop for TestReleaseGuard {
    fn drop(&mut self) {
        self.release();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn blocked_list_and_get_leave_ingress_responsive() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    atomic_write_json(&host.paths.manifest(manifest.terminal_id), &manifest).unwrap();
    lock_recover(&host.catalog).insert(&manifest).unwrap();
    let socket = host.socket_path().to_path_buf();
    let catalog = Arc::clone(&host.catalog);
    let (entered, mut operations) = tokio::sync::mpsc::unbounded_channel();
    *lock_recover(&host.catalog_test_probes.operations) = Some(entered);
    let (locked, locked_ready) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let blocker = std::thread::spawn(move || {
        let _catalog_guard = lock_recover(&catalog);
        locked.send(()).unwrap();
        released.recv_timeout(HOST_BLOCKING_TEST_TIMEOUT).unwrap();
    });
    locked_ready
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .unwrap();
    let serve = tokio::spawn(host.serve());
    wait_for_host_socket(&socket).await;
    let client = yolo_client::Client::new(&socket);
    let list_client = client.clone();
    let list = tokio::spawn(async move { list_client.list_sessions(false).await });
    let get_client = client.clone();
    let terminal_id = manifest.terminal_id;
    let get = tokio::spawn(async move { get_client.get_session(terminal_id).await });
    for _ in 0..CATALOG_OPERATION_COUNT {
        tokio::time::timeout(CONCURRENCY_OBSERVATION_TIMEOUT, operations.recv())
            .await
            .unwrap()
            .unwrap();
    }
    assert!(!list.is_finished());
    assert!(!get.is_finished());
    let status = tokio::time::timeout(HOST_STATUS_DEADLINE, client.status())
        .await
        .expect("host status must not wait for catalog clients")
        .unwrap();
    assert_eq!(status.home, directory.path());
    release.send(()).unwrap();
    blocker.join().unwrap();
    assert_eq!(list.await.unwrap().unwrap().len(), 1);
    assert_eq!(get.await.unwrap().unwrap().manifest.terminal_id, terminal_id);
    serve.abort();
    let _ = serve.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_watchdog_repair_is_fenced_by_the_next_host_generation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let stale = fixture_manifest(directory.path());
    host.paths.ensure_session(stale.terminal_id).unwrap();
    atomic_write_json(&host.paths.manifest(stale.terminal_id), &stale).unwrap();
    lock_recover(&host.catalog).insert(&stale).unwrap();
    let mut cancelled = stale.clone();
    cancelled.recovery_state = RecoveryState::Uncertain;
    cancelled.updated_at_ms = now_ms();
    host.write_namespace_json(&host.paths.manifest(cancelled.terminal_id), &cancelled)
        .unwrap();
    let (reached, repair_reached) = std::sync::mpsc::channel();
    let (release, repair_release) = std::sync::mpsc::channel();
    let mut repair_release_guard = TestReleaseGuard::new(release);
    let (completed, repair_completed) = std::sync::mpsc::channel();
    *lock_recover(&host.catalog_test_probes.scan) = Some(ReconcileScanProbe::repair(
        reached,
        repair_release,
        completed,
    ));
    let old_generation = host.controller_generation();
    let host = Arc::new(host);
    let watchdog = host.spawn_watchdog();
    tokio::task::spawn_blocking(move || {
        repair_reached
            .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
            .unwrap()
    })
    .await
    .unwrap();
    watchdog.cancel_and_wait().await;
    let mut current = cancelled;
    current.name = NEW_GENERATION_SESSION_NAME.into();
    current.updated_at_ms = now_ms();
    host.write_namespace_json(&host.paths.manifest(current.terminal_id), &current)
        .unwrap();
    host.catalog_upsert(&current).await.unwrap();
    drop(host);
    let (claim_reached, claimed) = std::sync::mpsc::channel();
    let (claim_release, open_release) = std::sync::mpsc::channel();
    let mut claim_release_guard = TestReleaseGuard::new(claim_release);
    let _claim_registration =
        install_controller_claim_probe(directory.path(), claim_reached.clone(), open_release);
    let home = directory.path().to_path_buf();
    let opening = std::thread::spawn(move || {
        let opened = Host::open(HostConfig::new(home));
        if let Err(error) = &opened {
            let _ = claim_reached.send(ControllerClaimProgress::OpenFailed(format!("{error:#}")));
        }
        opened
    });
    let progress =
        tokio::task::spawn_blocking(move || claimed.recv_timeout(CONTROLLER_CLAIM_TEST_TIMEOUT))
            .await
            .unwrap()
            .expect(CONTROLLER_CLAIM_TIMEOUT);
    let claimed_generation = match progress {
        ControllerClaimProgress::Claimed(generation) => generation,
        ControllerClaimProgress::OpenFailed(error) => {
            let _ = opening.join();
            panic!("{error}");
        }
    };
    assert!(claimed_generation > old_generation);
    repair_release_guard.release();
    let outcome = tokio::task::spawn_blocking(move || {
        repair_completed
            .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
            .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(outcome, CatalogRepairProbeOutcome::Fenced);
    claim_release_guard.release();
    let current_host = tokio::task::spawn_blocking(move || opening.join().unwrap().unwrap())
        .await
        .unwrap();
    assert_eq!(current_host.controller_generation(), claimed_generation);
    let terminal_id = current.terminal_id;
    let stored = current_host.catalog_get(terminal_id).await.unwrap().unwrap();
    assert_eq!(stored.terminal_id, terminal_id);
    assert_eq!(stored.name, NEW_GENERATION_SESSION_NAME);
}
