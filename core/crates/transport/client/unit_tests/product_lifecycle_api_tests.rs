use super::*;
use crate::protocol::method;
use api_matrix_support::*;

const PRODUCT_FORCE_DISCARD: bool = true;

#[tokio::test]
async fn conditional_product_lifecycle_uses_the_selected_session_wire_contract() {
    let terminal_id = test_session_id();
    let updated_at_ms = api_matrix_fixture_values::TEST_UPDATED_AT_MS;

    let terminate_server = MatrixServer::start(&[method::TERMINATE_SESSION]).await;
    let terminate_client = Client::new(terminate_server.socket.clone());
    terminate_server
        .rpc(terminate_client.terminate_if_current(
            terminal_id,
            false,
            updated_at_ms,
            TerminalRuntimeStatus::Running,
        ))
        .await
        .unwrap();
    let mut terminate_requests = terminate_server.finish().await;
    let terminate =
        terminate_requests.take_unique::<TerminalTerminateRequest>(method::TERMINATE_SESSION);
    assert_eq!(
        terminate,
        TerminalTerminateRequest {
            selection: TerminalLifecycleSelection::selected(
                terminal_id,
                updated_at_ms,
                TerminalRuntimeStatus::Running,
            ),
            force: false,
        }
    );
    terminate_requests.assert_empty();

    let archive_server = MatrixServer::start(&[method::ARCHIVE_SESSION]).await;
    let archive_client = Client::new(archive_server.socket.clone());
    let archived = archive_server
        .rpc(archive_client.archive_if_current(terminal_id, updated_at_ms, TerminalRuntimeStatus::Exited))
        .await
        .unwrap();
    assert_eq!(archived.manifest.terminal_id, terminal_id);
    let mut archive_requests = archive_server.finish().await;
    let archive = archive_requests.take_unique::<TerminalLifecycleSelection>(method::ARCHIVE_SESSION);
    assert_eq!(
        archive,
        TerminalLifecycleSelection::selected(terminal_id, updated_at_ms, TerminalRuntimeStatus::Exited)
    );
    archive_requests.assert_empty();

    let discard_server = MatrixServer::start(&[method::DISCARD_SESSION]).await;
    let discard_client = Client::new(discard_server.socket.clone());
    let discarded = discard_server
        .rpc(discard_client.discard_if_current(
            terminal_id,
            PRODUCT_FORCE_DISCARD,
            updated_at_ms,
            TerminalRuntimeStatus::Running,
        ))
        .await
        .unwrap();
    assert_eq!(discarded.manifest.terminal_id, terminal_id);
    let mut discard_requests = discard_server.finish().await;
    let discard = discard_requests.take_unique::<TerminalDiscardRequest>(method::DISCARD_SESSION);
    assert_eq!(
        discard,
        TerminalDiscardRequest {
            selection: TerminalLifecycleSelection::selected(
                terminal_id,
                updated_at_ms,
                TerminalRuntimeStatus::Running,
            ),
            force: PRODUCT_FORCE_DISCARD,
        }
    );
    discard_requests.assert_empty();
}

#[tokio::test]
async fn runtime_epoch_fenced_lifecycle_uses_a_separate_wire_precondition() {
    let terminal_id = test_session_id();
    let runtime_epoch = Uuid::new_v4();
    let server = MatrixServer::start(&[method::TERMINATE_SESSION, method::DISCARD_SESSION]).await;
    let client = Client::new(server.socket.clone());

    server
        .rpc(client.terminate_with_preconditions(
            TerminalLifecycleSelection::at_runtime_epoch(terminal_id, runtime_epoch),
            false,
        ))
        .await
        .unwrap();
    let discarded = server
        .rpc(client.discard_if_runtime_epoch(terminal_id, PRODUCT_FORCE_DISCARD, runtime_epoch))
        .await
        .unwrap();

    assert_eq!(discarded.manifest.terminal_id, terminal_id);
    let mut requests = server.finish().await;
    let terminate = requests.take_unique::<TerminalTerminateRequest>(method::TERMINATE_SESSION);
    assert_eq!(
        terminate.selection,
        TerminalLifecycleSelection::at_runtime_epoch(terminal_id, runtime_epoch)
    );
    let request = requests.take_unique::<TerminalDiscardRequest>(method::DISCARD_SESSION);
    assert_eq!(
        request,
        TerminalDiscardRequest {
            selection: TerminalLifecycleSelection::at_runtime_epoch(terminal_id, runtime_epoch),
            force: PRODUCT_FORCE_DISCARD,
        }
    );
    assert!(!request.selection.is_conditional());
    requests.assert_empty();
}
