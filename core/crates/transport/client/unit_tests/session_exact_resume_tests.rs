use super::api_matrix_support::*;
use super::*;
use crate::protocol::method;

#[tokio::test]
async fn ordinary_resume_uses_the_same_current_method_as_exact_recreation() {
    let server = MatrixServer::start(&[method::RESUME_SESSION_EXACT]).await;
    let client = Client::new(server.socket.clone());
    let session = test_session_id();
    server.rpc(client.resume(session, false)).await.unwrap();
    let mut requests = server.finish().await;
    let request = requests.take_unique::<ResumeRequest>(method::RESUME_SESSION_EXACT);
    assert_eq!(request.selection, TerminalLifecycleSelection::unconditional(session));
    assert!(!request.recreate);
    requests.assert_empty();
}

#[tokio::test]
async fn exact_recreate_fails_closed_against_a_legacy_host() {
    let server = MatrixServer::start_with_scenario(
        &[method::RESUME_SESSION_EXACT],
        MatrixScenario::LegacyResumeHost,
    )
    .await;
    let client = Client::new(server.socket.clone());
    let selection = TerminalLifecycleSelection::selected(
        test_session_id(),
        api_matrix_fixture_values::TEST_UPDATED_AT_MS,
        TerminalRuntimeStatus::Unreachable,
    )
    .with_expected_manifest_sha256(Some(api_matrix_fixtures::test_manifest_digest()))
    .with_expected_runtime_epoch(Some(test_runtime_epoch()));

    assert!(matches!(
        server
            .rpc(client.resume_with_preconditions(selection.clone(), true))
            .await,
        Err(ClientError::Remote { code, .. })
            if code == crate::protocol::error_code::UNSUPPORTED_METHOD
    ));
    let mut requests = server.finish().await;
    let request = requests.take_unique::<ResumeRequest>(method::RESUME_SESSION_EXACT);
    assert_eq!(request.selection, selection);
    assert!(request.recreate);
    requests.assert_empty();
}
