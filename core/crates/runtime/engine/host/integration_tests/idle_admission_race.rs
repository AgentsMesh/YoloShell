use crate::host::tests::*;
use super::*;

const IDLE_ADMISSION_SOCKET: &str = "idle-admission.sock";
const IDLE_ADMISSION_CHECK_NOT_STARTED: &str = "idle check did not enter its drain";
const IDLE_ADMISSION_RESPONSE_TIMEOUT: &str = "accepted client did not receive a Host response";
const IDLE_ADMISSION_TRANSPORT_CLOSED: &str = "idle cancellation closed the accepted transport";

#[tokio::test(start_paused = true)]
async fn new_client_during_quiescent_verification_keeps_transport_open() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let reconcile = Arc::clone(&host.reconcile_gate).lock_owned().await;
    let socket = directory.path().join(IDLE_ADMISSION_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let (armed, ready) = tokio::sync::oneshot::channel();
    let mut ingress = HostIngress::spawn(Arc::clone(&host), listener, armed);
    ready.await.unwrap();
    drop(idle_admission_status_client(&socket).await);

    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, async {
        while !host.request_drain.is_draining() {
            tokio::time::sleep(HOST_IDLE_SHUTDOWN_QUIET_PERIOD).await;
        }
    })
    .await
    .expect(IDLE_ADMISSION_CHECK_NOT_STARTED);

    let replacement = idle_admission_status_client(&socket).await;
    assert!(!host.request_drain.is_draining());
    assert!(host.request_drain.admit_mutation().await.is_ok());
    drop(reconcile);
    drop(replacement);
    ingress.shutdown().await.unwrap();
}

async fn idle_admission_status_client(socket: &Path) -> yolo_capnp_rpc::Connection {
    let client = UnixStream::connect(socket).await.unwrap();
    let client = open_host_rpc(client).await;
    let response: ResponseEnvelope = tokio::time::timeout(
        HOST_BLOCKING_TEST_TIMEOUT,
        host_rpc(&client, &request(method::HOST_STATUS, &())),
    )
    .await
    .expect(IDLE_ADMISSION_RESPONSE_TIMEOUT);
    assert!(response.ok);
    client
}
