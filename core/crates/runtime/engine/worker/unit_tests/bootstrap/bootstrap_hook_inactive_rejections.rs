use crate::worker::tests::*;
#[tokio::test(flavor = "current_thread")]
async fn inactive_hook_rejects_wrong_schema_and_lifecycle_without_authority() {
    let fixture = Fixture::new();
    let server = hook_pump_server(&fixture).await;
    let socket = server.environment().socket_path().to_path_buf();
    let mut tasks = JoinSet::new();
    let mut pump = claude_hook::spawn_admission_pump(
        &mut tasks,
        server,
        Arc::clone(&fixture.state),
        hook_pump_shell(&fixture),
    );
    pump.wait_ready().await.unwrap();
    let mut wrong_schema = hook_gate_request();
    wrong_schema.schema_version += 1;
    let mut lifecycle = hook_gate_request();
    lifecycle.event = crate::claude_hooks::ClaudeLifecycleEvent::SessionEnd {
        event_id: Uuid::new_v4(),
        provider_session_id: HOOK_GATE_AGENT_SESSION.into(),
        reason: crate::claude_hooks::ClaudeSessionEndReason::Other,
    };
    for request in [wrong_schema, lifecycle] {
        let response = tokio::time::timeout(
            HOOK_GATE_SETTLEMENT_TIMEOUT,
            hook_gate_exchange(&socket, &request),
        )
        .await
        .unwrap();
        assert_hook_gate_rejection(&response, crate::claude_hooks::CLAUDE_HOOK_REJECTED);
    }
    assert!(
        !fixture
            .state
            .paths
            .agent_binding(fixture.terminal_id())
            .exists()
    );
    abort_and_drain_tasks(&mut tasks).await;
}

pub(super) async fn hook_gate_exchange<T: serde::Serialize>(
    socket: &std::path::Path,
    request: &T,
) -> crate::claude_hooks::ClaudeNativeResponse {
    let mut client = crate::local_unix::LocalUnixStream::connect(socket)
        .await
        .unwrap();
    crate::claude_hooks::write_frame(&mut client, request)
        .await
        .unwrap();
    crate::claude_hooks::read_frame(&mut client).await.unwrap()
}

pub(super) fn assert_hook_gate_rejection(response: &crate::claude_hooks::ClaudeNativeResponse, code: &str) {
    assert_eq!(
        response.schema_version,
        crate::claude_hooks::CLAUDE_NATIVE_SCHEMA_VERSION
    );
    assert!(!response.accepted);
    assert_eq!(response.code, code);
    assert!(response.revision.is_none());
    assert!(response.invocation_id.is_none());
}
