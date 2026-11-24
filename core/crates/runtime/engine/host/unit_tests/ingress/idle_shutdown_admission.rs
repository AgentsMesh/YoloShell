use crate::host::tests::*;
#[tokio::test]
async fn accepted_client_cancels_quiescent_shutdown_and_reopens_admission() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let clients = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CLIENTS));
    let client = std::sync::Arc::clone(&clients).try_acquire_owned().unwrap();

    assert!(host.prepare_quiescent_shutdown(clients).await.is_err());
    assert!(!host.request_drain.is_draining());
    assert!(host.request_drain.admit_mutation().await.is_ok());
    drop(client);
}
const IDLE_SHUTDOWN_EXITED_SOCKET: &str = "exited.sock";

const ARCHIVED_SETTLED_SOCKET: &str = "archived-settled.sock";
const ARCHIVED_LIVE_SOCKET: &str = "archived-live.sock";
