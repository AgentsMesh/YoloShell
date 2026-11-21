use crate::host::tests::*;
use super::*;

const STARTUP_SOCKET_TIMEOUT: Duration = HOST_BLOCKING_TEST_TIMEOUT;
pub(super) const STARTUP_SOCKET_POLL_INTERVAL: Duration = Duration::from_millis(5);
pub(super) const HOST_STATUS_DEADLINE: Duration = Duration::from_millis(250);
const STALLED_STATUS_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const CONCURRENCY_OBSERVATION_TIMEOUT: Duration = HOST_BLOCKING_TEST_TIMEOUT;
const NO_OVERLAP_OBSERVATION: Duration = Duration::from_millis(100);
const RECONCILE_BATCH_COUNT: usize = 2;
const STALLED_SESSION_COUNT: usize = MAX_RECONCILE_CONCURRENCY * RECONCILE_BATCH_COUNT;
const RECONCILE_GROUP_SIZE: usize = STALLED_SESSION_COUNT / RECONCILE_BATCH_COUNT;
const NEWEST_GROUP_UPDATED_AT_MS: i64 = 2;
const OLDEST_GROUP_UPDATED_AT_MS: i64 = 1;
const STALLED_SOCKET_PREFIX: &str = "startup-stalled-";
const STALLED_SOCKET_SUFFIX: &str = ".sock";

struct StalledSessions {
    ids: Vec<Uuid>,
    servers: Vec<tokio::task::JoinHandle<()>>,
}

pub(super) async fn wait_for_host_socket(path: &Path) {
    tokio::time::timeout(STARTUP_SOCKET_TIMEOUT, async {
        loop {
            match UnixStream::connect(path).await {
                Ok(stream) => return drop(stream),
                Err(_) => tokio::time::sleep(STARTUP_SOCKET_POLL_INTERVAL).await,
            }
        }
    })
    .await
    .unwrap();
}

fn install_stalled_sessions(
    host: &Host,
    root: &Path,
    group: &str,
    updated_at_ms: i64,
    started: tokio::sync::mpsc::UnboundedSender<Uuid>,
) -> StalledSessions {
    let mut ids = Vec::with_capacity(RECONCILE_GROUP_SIZE);
    let mut servers = Vec::with_capacity(RECONCILE_GROUP_SIZE);
    for index in 0..RECONCILE_GROUP_SIZE {
        let mut manifest = fixture_manifest(root);
        manifest.updated_at_ms = updated_at_ms;
        let socket = root.join(format!(
            "{STALLED_SOCKET_PREFIX}{group}-{index}{STALLED_SOCKET_SUFFIX}"
        ));
        let listener = UnixListener::bind(&socket).unwrap();
        let record = host_runtime_record(host, manifest.terminal_id, socket);
        ids.push(manifest.terminal_id);
        persist_host_session(host, &manifest, &record);
        let started = started.clone();
        servers.push(tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = UnixStream::from(stream);
            let mut stream = accept_worker_auth(stream, &record).await;
            let _: RequestEnvelope = stream.request().await.unwrap();
            started.send(record.terminal_id).unwrap();
            std::future::pending::<()>().await;
        }));
    }
    StalledSessions { ids, servers }
}

#[tokio::test]
async fn host_accepts_status_while_initial_reconcile_is_stalled_and_single_flight() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.worker_status_timeout = STALLED_STATUS_TIMEOUT;
    let socket = host.socket_path().to_path_buf();
    let (started, mut starts) = tokio::sync::mpsc::unbounded_channel();
    let oldest = install_stalled_sessions(
        &host,
        directory.path(),
        "oldest",
        OLDEST_GROUP_UPDATED_AT_MS,
        started.clone(),
    );
    let newest = install_stalled_sessions(
        &host,
        directory.path(),
        "newest",
        NEWEST_GROUP_UPDATED_AT_MS,
        started,
    );
    let catalog = Arc::clone(&host.catalog);
    let serve = tokio::spawn(host.serve());
    wait_for_host_socket(&socket).await;

    for _ in 0..MAX_RECONCILE_CONCURRENCY {
        tokio::time::timeout(CONCURRENCY_OBSERVATION_TIMEOUT, starts.recv())
            .await
            .unwrap()
            .unwrap();
    }
    for terminal_id in newest.ids {
        lock_recover(&catalog).archive(terminal_id, true).unwrap();
    }
    let client = yolo_client::Client::new(&socket);
    let manual_client = client.clone();
    let manual = tokio::spawn(async move { manual_client.reconcile().await });
    assert!(
        tokio::time::timeout(NO_OVERLAP_OBSERVATION, starts.recv())
            .await
            .is_err(),
        "reconcile sweeps must be bounded and single-flight"
    );
    let status = tokio::time::timeout(HOST_STATUS_DEADLINE, client.status())
        .await
        .expect("host status must not wait for recovery")
        .unwrap();
    assert_eq!(status.home, directory.path());

    manual.abort();
    serve.abort();
    let _ = manual.await;
    let _ = serve.await;
    for server in newest.servers.into_iter().chain(oldest.servers) {
        server.abort();
        let _ = server.await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn catalog_scan_never_blocks_an_armed_ingress() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = host.socket_path().to_path_buf();
    let catalog = Arc::clone(&host.catalog);
    let (scan_probe, scan_started) = tokio::sync::oneshot::channel();
    *lock_recover(&host.catalog_test_probes.scan) = Some(ReconcileScanProbe::started(scan_probe));
    let (locked, locked_ready) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let blocker = std::thread::spawn(move || {
        let _catalog_guard = lock_recover(&catalog);
        locked.send(()).unwrap();
        released.recv().unwrap();
    });
    locked_ready.recv().unwrap();
    let serve = tokio::spawn(host.serve());
    scan_started.await.unwrap();
    wait_for_host_socket(&socket).await;

    let client = yolo_client::Client::new(&socket);
    let status = tokio::time::timeout(HOST_STATUS_DEADLINE, client.status())
        .await
        .expect("host status must not wait for catalog recovery")
        .unwrap();
    assert_eq!(status.home, directory.path());

    release.send(()).unwrap();
    blocker.join().unwrap();
    serve.abort();
    let _ = serve.await;
}
