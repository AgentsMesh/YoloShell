use super::*;
use crate::protocol::{to_value, write_frame};
use tempfile::tempdir;

const HOST_RESPONSE_DRAIN_TEST_DETAIL: &str = "response drain test fail-stop";
const HOST_RESPONSE_DRAIN_TEST_TIMEOUT: Duration = Duration::from_secs(1);
const HOST_RESPONSE_DRAIN_BLOCKING_TIMEOUT: Duration = Duration::from_secs(10);

const RESPONSE_DRAIN_SOCKET_REQUEST_ID: &str = "response-drain-socket-request";

struct ResponseDrainCatalogRelease(Option<std::sync::mpsc::Sender<()>>);

impl ResponseDrainCatalogRelease {
    fn release(&mut self) {
        if let Some(release) = self.0.take() {
            release.send(()).unwrap();
        }
    }
}

impl Drop for ResponseDrainCatalogRelease {
    fn drop(&mut self) {
        if let Some(release) = self.0.take() {
            let _ = release.send(());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_response_drain_preserves_socket_response_after_dispatch_unblocks() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let catalog = Arc::clone(&host.catalog);
    let (catalog_locked, locked) = std::sync::mpsc::channel();
    let (release_catalog, released) = std::sync::mpsc::channel();
    let mut release = ResponseDrainCatalogRelease(Some(release_catalog));
    let blocker = std::thread::spawn(move || {
        let _guard = lock_recover(&catalog);
        catalog_locked.send(()).unwrap();
        released
            .recv_timeout(HOST_RESPONSE_DRAIN_BLOCKING_TIMEOUT)
            .unwrap();
    });
    locked
        .recv_timeout(HOST_RESPONSE_DRAIN_BLOCKING_TIMEOUT)
        .unwrap();
    let (operation_started, mut operations) = tokio::sync::mpsc::unbounded_channel();
    *lock_recover(&host.catalog_test_probes.operations) = Some(operation_started);
    let (mut client, server) = UnixStream::pair().unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    let client_host = Arc::clone(&host);
    tasks.spawn(async move {
        let _ = client_host.handle_client(server).await;
    });
    let request = RequestEnvelope::new(
        RESPONSE_DRAIN_SOCKET_REQUEST_ID,
        method::LIST_SESSIONS,
        to_value(&ListSessionsRequest {
            include_archived: false,
            require_complete: true,
        })
        .unwrap(),
    );
    let client = yolo_capnp_rpc::Connection::open(
        client,
        yolo_capnp_rpc::Protocol::Endpoint,
        tokio::time::Instant::now() + HOST_RESPONSE_DRAIN_TEST_TIMEOUT,
    )
    .await
    .unwrap();
    let pending = tokio::spawn(async move {
        client
            .request(
                yolo_capnp_rpc::Method::Call,
                yolo_capnp_codec::to_value(&request).unwrap(),
                tokio::time::Instant::now() + HOST_RESPONSE_DRAIN_BLOCKING_TIMEOUT,
            )
            .await
    });
    tokio::time::timeout(HOST_RESPONSE_DRAIN_TEST_TIMEOUT, operations.recv())
        .await
        .unwrap()
        .unwrap();
    host.request_drain
        .fail_stop(HOST_RESPONSE_DRAIN_TEST_DETAIL);
    let draining_host = Arc::clone(&host);
    let draining = tokio::spawn(async move {
        drain_host_clients(&draining_host, &mut tasks).await;
    });
    tokio::task::yield_now().await;
    assert!(!draining.is_finished());

    release.release();
    blocker.join().unwrap();
    let response = tokio::time::timeout(HOST_RESPONSE_DRAIN_TEST_TIMEOUT, pending)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let response: ResponseEnvelope = yolo_capnp_codec::from_value(response.value).unwrap();

    assert!(response.ok);
    assert_eq!(response.request_id, RESPONSE_DRAIN_SOCKET_REQUEST_ID);
    assert_eq!(response.method, method::LIST_SESSIONS);
    tokio::time::timeout(HOST_RESPONSE_DRAIN_TEST_TIMEOUT, draining)
        .await
        .unwrap()
        .unwrap();
}
