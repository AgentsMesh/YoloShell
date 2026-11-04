use super::*;
use crate::protocol::method;
use api_matrix_support::*;

const TEST_CREATION_TOKEN_VALUE: u128 = 97;
const TEST_SESSION_NAME: &str = "session-api-matrix";
const TEST_SESSION_CWD: &str = "/tmp/yolo-session-api";
const TEST_CREATE_COLUMNS: u16 = 109;
const TEST_CREATE_ROWS: u16 = 113;
const SESSION_METHODS: &[&str] = &[
    method::HOST_STATUS,
    method::LIST_SESSIONS,
    method::GET_SESSION,
    method::CREATE_SESSION,
    method::TERMINATE_SESSION,
    method::ARCHIVE_SESSION,
    method::DISCARD_SESSION,
    method::CLEANUP_TERMINATE_SESSION,
    method::CLEANUP_ARCHIVE_SESSION,
    method::CLEANUP_DISCARD_SESSION,
    method::FINALIZE_CREATION,
    method::RESUME_SESSION_EXACT,
    method::RECONCILE,
];
const CONCURRENT_METHODS: &[&str] = &[
    method::LIST_SESSIONS,
    method::LIST_SESSIONS,
    method::LIST_SESSIONS,
];
const INCLUDE_ARCHIVED: bool = true;
const EXCLUDE_ARCHIVED: bool = false;

#[tokio::test]
async fn every_session_client_api_uses_its_complete_typed_wire_contract() {
    let server = MatrixServer::start(SESSION_METHODS).await;
    let client = Client::new(server.socket.clone());
    let terminal_id = test_session_id();
    let capability = test_creation_capability();
    let creation_token = Uuid::from_u128(TEST_CREATION_TOKEN_VALUE);
    let create = CreateTerminal {
        creation_token,
        name: Some(TEST_SESSION_NAME.into()),
        cwd: PathBuf::from(TEST_SESSION_CWD),
        cols: Some(TEST_CREATE_COLUMNS),
        rows: Some(TEST_CREATE_ROWS),
        initial_input: None,
        context: None,
    };

    server.rpc(client.status()).await.unwrap();
    let listed = server
        .rpc(client.list_sessions(INCLUDE_ARCHIVED))
        .await
        .unwrap();
    assert!(matches!(
        listed.as_slice(),
        [view] if view.manifest.terminal_id == terminal_id
    ));
    let fetched = server.rpc(client.get_session(terminal_id)).await.unwrap();
    assert_eq!(fetched.manifest.terminal_id, terminal_id);
    let created = server.rpc(client.create_session(&create)).await.unwrap();
    assert_eq!(created.terminal_id, terminal_id);
    server
        .rpc(client.terminate(terminal_id, true))
        .await
        .unwrap();
    server.rpc(client.archive(terminal_id)).await.unwrap();
    server.rpc(client.discard(terminal_id, true)).await.unwrap();
    let cleanup = server
        .rpc(client.cleanup_terminate(terminal_id, capability, true))
        .await
        .unwrap();
    assert_eq!(cleanup, CleanupTerminationOutcome::Terminated);
    server
        .rpc(client.cleanup_archive(terminal_id, capability))
        .await
        .unwrap();
    server
        .rpc(client.cleanup_discard(terminal_id, capability, true))
        .await
        .unwrap();
    server
        .rpc(client.finalize_creation(creation_token, terminal_id, capability))
        .await
        .unwrap();
    server.rpc(client.resume(terminal_id, true)).await.unwrap();
    server.rpc(client.reconcile()).await.unwrap();

    session_api_matrix_assertions::assert_session_requests(
        server.finish().await,
        terminal_id,
        capability,
        creation_token,
        &create,
    );
}

#[tokio::test]
async fn snapshot_fails_closed_for_missing_or_mismatched_runtime_identity() {
    let unavailable = MatrixServer::start_with_scenario(
        &[method::GET_SESSION],
        MatrixScenario::RuntimeUnavailable,
    )
    .await;
    let client = Client::new(unavailable.socket.clone());
    assert!(matches!(
        unavailable.rpc(client.snapshot(test_session_id())).await,
        Err(ClientError::RuntimeUnavailable { terminal_id }) if terminal_id == test_session_id()
    ));
    let mut requests = unavailable.finish().await;
    let get = requests.take_unique::<TerminalRef>(method::GET_SESSION);
    assert_eq!(get.terminal_id, test_session_id());
    requests.assert_empty();

    let mismatch = MatrixServer::start_with_scenario(
        &[method::GET_SESSION, method::GET_SNAPSHOT],
        MatrixScenario::MismatchedSnapshot,
    )
    .await;
    let client = Client::new(mismatch.socket.clone());
    assert!(matches!(
        mismatch.rpc(client.snapshot(test_session_id())).await,
        Err(ClientError::Protocol(ProtocolError::AttachmentMismatch))
    ));
    assert_snapshot_requests(mismatch.finish().await);
}

#[tokio::test]
async fn complete_listing_rejects_a_response_without_completeness_attestation() {
    let server = MatrixServer::start_with_scenario(
        &[method::LIST_SESSIONS],
        MatrixScenario::IncompleteCatalogInventory,
    )
    .await;
    let client = Client::new(server.socket.clone());

    assert!(matches!(
        server
            .rpc(client.list_sessions_complete(INCLUDE_ARCHIVED))
            .await,
        Err(ClientError::IncompleteCatalogInventory)
    ));
    let mut requests = server.finish().await;
    let request = requests.take_unique::<ListSessionsRequest>(method::LIST_SESSIONS);
    assert!(request.include_archived);
    assert!(request.require_complete);
    requests.assert_empty();
}

#[tokio::test]
async fn cloned_client_handles_concurrent_requests_without_accept_order_assumptions() {
    let server = MatrixServer::start(CONCURRENT_METHODS).await;
    let client = Client::new(server.socket.clone());
    let archived_client = client.clone();
    let complete_client = client.clone();
    let (active, archived, complete) = tokio::join!(
        server.rpc(client.list_sessions(EXCLUDE_ARCHIVED)),
        server.rpc(archived_client.list_sessions(INCLUDE_ARCHIVED)),
        server.rpc(complete_client.list_sessions_complete(INCLUDE_ARCHIVED)),
    );
    active.unwrap();
    archived.unwrap();
    complete.unwrap();

    let mut requests = server.finish().await;
    let active = requests
        .take_by_typed_key::<ListSessionsRequest, _>(method::LIST_SESSIONS, |request| {
            request.include_archived == EXCLUDE_ARCHIVED && !request.require_complete
        });
    assert_eq!(active.include_archived, EXCLUDE_ARCHIVED);
    let archived = requests
        .take_by_typed_key::<ListSessionsRequest, _>(method::LIST_SESSIONS, |request| {
            !request.require_complete
        });
    assert_eq!(archived.include_archived, INCLUDE_ARCHIVED);
    let complete = requests.take_unique::<ListSessionsRequest>(method::LIST_SESSIONS);
    assert!(complete.include_archived);
    assert!(complete.require_complete);
    requests.assert_empty();
}

fn assert_snapshot_requests(mut requests: MatrixRequests) {
    let get = requests.take_unique::<TerminalRef>(method::GET_SESSION);
    assert_eq!(get.terminal_id, test_session_id());
    let snapshot = requests.take_unique::<SnapshotRequest>(method::GET_SNAPSHOT);
    assert_eq!(snapshot.terminal_id, test_session_id());
    assert_eq!(snapshot.runtime_epoch, test_target().runtime_epoch);
    requests.assert_empty();
}
