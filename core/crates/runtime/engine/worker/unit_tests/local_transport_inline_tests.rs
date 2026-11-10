use super::*;

const SERVER_START_TIME_OFFSET: u64 = 1;

#[cfg(feature = "yolo-unit-tests")]
#[tokio::test]
async fn server_verification_accepts_the_exact_connected_runtime() {
    let (server, client) = LocalUnixStream::pair().unwrap();
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    verify_server_identity(&client, identity).unwrap();
    verify_same_uid(&server).unwrap();
}

#[cfg(feature = "yolo-unit-tests")]
#[tokio::test]
async fn server_verification_rejects_another_runtime_identity() {
    let (_server, client) = LocalUnixStream::pair().unwrap();
    let mut identity = ProcessIdentity::capture(std::process::id()).unwrap();
    identity.start_time = identity.start_time.saturating_add(SERVER_START_TIME_OFFSET);
    let error = verify_server_identity(&client, identity).unwrap_err();
    assert!(matches!(error, WorkerRpcError::IdentityMismatch(_)));
}
