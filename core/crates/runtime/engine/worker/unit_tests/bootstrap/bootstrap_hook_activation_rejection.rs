use super::*;
use crate::claude_hooks::{
    CLAUDE_HOOK_REJECTED, CLAUDE_NATIVE_SCHEMA_VERSION, ClaudeHookState, ClaudeLifecycleEvent,
    ClaudeNativeRequest, ClaudeNativeResponse, ClaudeSessionEndReason,
};

const HOOK_GATE_SCHEMA_OFFSET: u32 = 1;

#[tokio::test(flavor = "current_thread")]
async fn inactive_hook_rejects_zero_previous_and_future_native_schemas() {
    let requests = [
        u32::default(),
        CLAUDE_NATIVE_SCHEMA_VERSION - HOOK_GATE_SCHEMA_OFFSET,
        CLAUDE_NATIVE_SCHEMA_VERSION + HOOK_GATE_SCHEMA_OFFSET,
    ]
    .map(|schema_version| ClaudeNativeRequest {
        schema_version,
        ..hook_gate_request()
    });
    assert_inactive_hook_rejections(&requests).await;
}

#[tokio::test(flavor = "current_thread")]
async fn inactive_hook_rejects_every_lifecycle_event() {
    let cwd = std::path::PathBuf::from(HOOK_GATE_PROJECT);
    let transcript_path = cwd.join(HOOK_GATE_TRANSCRIPT_FILE);
    let events = [
        ClaudeLifecycleEvent::StateChanged {
            event_id: Uuid::new_v4(),
            provider_session_id: HOOK_GATE_AGENT_SESSION.to_owned(),
            state: ClaudeHookState::WaitingForInput,
        },
        ClaudeLifecycleEvent::CwdChanged {
            event_id: Uuid::new_v4(),
            provider_session_id: HOOK_GATE_AGENT_SESSION.to_owned(),
            cwd,
            transcript_path,
        },
        ClaudeLifecycleEvent::SessionEnd {
            event_id: Uuid::new_v4(),
            provider_session_id: HOOK_GATE_AGENT_SESSION.to_owned(),
            reason: ClaudeSessionEndReason::Other,
        },
    ];
    let requests = events.map(|event| ClaudeNativeRequest {
        event,
        ..hook_gate_request()
    });
    assert_inactive_hook_rejections(&requests).await;
}

#[tokio::test(flavor = "current_thread")]
async fn inactive_hook_rejects_malformed_request_shapes() {
    assert_inactive_hook_rejections(&[
        serde_json::Value::Null,
        serde_json::Value::Array(Vec::new()),
    ])
    .await;
}

async fn assert_inactive_hook_rejections<Request: serde::Serialize>(requests: &[Request]) {
    let fixture = Fixture::new();
    let server = hook_pump_server(&fixture).await;
    let socket = server.environment().socket_path().to_path_buf();
    let runtime = fixture.state.paths.runtime_record(fixture.terminal_id());
    let runtime_before = std::fs::read(&runtime).unwrap();
    let binding = fixture.state.paths.agent_binding(fixture.terminal_id());
    assert!(!binding.exists());
    let mut tasks = JoinSet::new();
    let mut pump = claude_hook::spawn_admission_pump(
        &mut tasks,
        server,
        Arc::clone(&fixture.state),
        hook_pump_shell(&fixture),
    );
    pump.wait_ready().await.unwrap();
    for reported_ready in [false, true] {
        fixture.state.ready.store(reported_ready, Ordering::Release);
        for request in requests {
            let mut client = crate::local_unix::LocalUnixStream::connect(&socket)
                .await
                .unwrap();
            crate::claude_hooks::write_frame(&mut client, request)
                .await
                .unwrap();
            let response = tokio::time::timeout(
                HOOK_GATE_SETTLEMENT_TIMEOUT,
                crate::claude_hooks::read_frame::<_, ClaudeNativeResponse>(&mut client),
            )
            .await
            .unwrap();
            match response {
                Ok(response) => assert_hook_gate_rejection(&response, CLAUDE_HOOK_REJECTED),
                Err(_) => assert!(
                    serde_json::from_value::<ClaudeNativeRequest>(
                        serde_json::to_value(request).unwrap()
                    )
                    .is_err()
                ),
            }
            assert!(!binding.exists());
            assert_eq!(std::fs::read(&runtime).unwrap(), runtime_before);
        }
    }
    abort_and_drain_tasks(&mut tasks).await;
}
