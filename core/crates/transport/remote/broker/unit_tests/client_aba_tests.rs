use super::client::{BrokerClient, BrokerClientError, BrokerTransition};
use super::constants::PRIVATE_FILE_MODE;
use super::protocol::{
    BrokerRequest, BrokerRequestEnvelope, BrokerResponse, BrokerResult, BrokerStatus,
};
use super::socket_identity::socket_identity;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tokio::net::UnixListener;
use uuid::Uuid;

const MANAGEMENT_SOCKET: &str = "management.sock";
const HOST_ID: &str = "aba-host";
const OBSERVED_REVISION: &str = "observed-revision";
const DESIRED_REVISION: &str = "desired-revision";
const CONNECTION_GENERATION: u64 = 7;
const REPLACEMENT_ACCEPT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(50);
const CLIENT_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

#[tokio::test]
async fn status_and_shutdown_stay_on_one_connection_after_path_replacement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(MANAGEMENT_SOCKET);
    let listener = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let expected_socket = socket_identity(&path).unwrap();
    let instance_id = Uuid::new_v4();
    let server_path = path.clone();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let status_request: BrokerRequestEnvelope =
            crate::protocol::read_frame(&mut stream).await.unwrap();
        assert_eq!(status_request.request, BrokerRequest::Status);
        let status = BrokerStatus::disconnected(
            instance_id,
            HOST_ID.into(),
            OBSERVED_REVISION.into(),
            CONNECTION_GENERATION,
        );
        crate::protocol::write_frame(
            &mut stream,
            &BrokerResponse::success(status_request.request_id, BrokerResult::Status(status)),
        )
        .await
        .unwrap();

        fs::remove_file(&server_path).unwrap();
        let replacement = UnixListener::bind(&server_path).unwrap();
        fs::set_permissions(&server_path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        let shutdown: BrokerRequestEnvelope =
            crate::protocol::read_frame(&mut stream).await.unwrap();
        assert_eq!(
            shutdown.request,
            BrokerRequest::Shutdown {
                expected_revision: OBSERVED_REVISION.into(),
                expected_instance_id: instance_id,
            }
        );
        crate::protocol::write_frame(
            &mut stream,
            &BrokerResponse::success(
                shutdown.request_id,
                BrokerResult::Shutdown {
                    config_revision: OBSERVED_REVISION.into(),
                    instance_id,
                },
            ),
        )
        .await
        .unwrap();
        assert!(
            tokio::time::timeout(REPLACEMENT_ACCEPT_TIMEOUT, replacement.accept())
                .await
                .is_err()
        );
    });

    let transition = BrokerClient::new(path)
        .transition_at(
            DESIRED_REVISION,
            expected_socket,
            tokio::time::Instant::now() + CLIENT_REQUEST_TIMEOUT,
        )
        .await
        .unwrap();

    assert_eq!(transition, BrokerTransition::Shutdown);
    server.await.unwrap();
}

#[tokio::test]
async fn current_status_rejects_path_replacement_before_returning() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(MANAGEMENT_SOCKET);
    let listener = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let expected_socket = socket_identity(&path).unwrap();
    let server_path = path.clone();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request: BrokerRequestEnvelope =
            crate::protocol::read_frame(&mut stream).await.unwrap();
        fs::remove_file(&server_path).unwrap();
        let _replacement = UnixListener::bind(&server_path).unwrap();
        fs::set_permissions(&server_path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        let status = BrokerStatus::disconnected(
            Uuid::new_v4(),
            HOST_ID.into(),
            DESIRED_REVISION.into(),
            CONNECTION_GENERATION,
        );
        crate::protocol::write_frame(
            &mut stream,
            &BrokerResponse::success(request.request_id, BrokerResult::Status(status)),
        )
        .await
        .unwrap();
    });

    let error = BrokerClient::new(path)
        .transition_at(
            DESIRED_REVISION,
            expected_socket,
            tokio::time::Instant::now() + CLIENT_REQUEST_TIMEOUT,
        )
        .await
        .unwrap_err();

    assert!(matches!(error, BrokerClientError::UnsafeSocket(_)));
    server.await.unwrap();
}

#[tokio::test]
async fn silent_broker_obeys_the_callers_absolute_deadline() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(MANAGEMENT_SOCKET);
    let listener = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let expected_socket = socket_identity(&path).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _: BrokerRequestEnvelope = crate::protocol::read_frame(&mut stream).await.unwrap();
        std::future::pending::<()>().await;
    });

    let error = BrokerClient::new(path)
        .transition_at(
            DESIRED_REVISION,
            expected_socket,
            tokio::time::Instant::now() + CLIENT_REQUEST_TIMEOUT,
        )
        .await
        .unwrap_err();

    assert!(matches!(error, BrokerClientError::Deadline));
    server.abort();
    let _ = server.await;
}
