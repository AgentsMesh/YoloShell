use crate::worker::tests::*;
pub(super) const HOOK_GATE_AGENT_SESSION: &str = "activation-gate-agent";
const HOOK_GATE_PROJECT: &str = "/tmp/yolo-hook-gate-project";
const HOOK_GATE_AGENT_PID: u32 = 56_003;
const HOOK_GATE_AGENT_START: u64 = 56_004;
const HOOK_GATE_TRANSCRIPT_FILE: &str = "activation-gate-transcript.jsonl";
pub(super) const HOOK_GATE_SETTLEMENT_TIMEOUT: Duration = WORKER_TERMINATION_TIMEOUT;

#[path = "bootstrap_hook_activation_rejection.rs"]
mod hook_activation_rejection;

#[tokio::test(flavor = "current_thread")]
async fn inactive_hook_reports_not_ready_without_applying_a_transaction() {
    assert_hook_activation_gate(false).await;
}

#[tokio::test(flavor = "current_thread")]
async fn early_ready_flag_cannot_bypass_explicit_hook_activation() {
    assert_hook_activation_gate(true).await;
}

async fn assert_hook_activation_gate(reported_ready: bool) {
    let fixture = Fixture::new();
    fixture.state.ready.store(reported_ready, Ordering::Release);
    let server = hook_pump_server(&fixture).await;
    let socket = server.environment().socket_path().to_path_buf();
    let mut client = crate::local_unix::LocalUnixStream::connect(&socket)
        .await
        .unwrap();
    let mut tasks = JoinSet::new();
    let mut pump = claude_hook::spawn_admission_pump(
        &mut tasks,
        server,
        Arc::clone(&fixture.state),
        hook_pump_shell(&fixture),
    );
    pump.wait_ready().await.unwrap();
    crate::claude_hooks::write_frame(&mut client, &hook_gate_request())
        .await
        .unwrap();
    let pending = tokio::time::timeout(
        HOOK_GATE_SETTLEMENT_TIMEOUT,
        crate::claude_hooks::read_frame::<_, crate::claude_hooks::ClaudeNativeResponse>(
            &mut client,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!pending.accepted);
    assert_eq!(pending.code, crate::claude_hooks::CLAUDE_HOOK_NOT_READY);
    assert!(pending.revision.is_none());
    assert!(pending.invocation_id.is_none());
    assert!(
        !fixture
            .state
            .paths
            .agent_binding(fixture.terminal_id())
            .exists()
    );
    fixture.state.ready.store(true, Ordering::Release);
    pump.activate().unwrap();
    let response = tokio::time::timeout(HOOK_GATE_SETTLEMENT_TIMEOUT, async {
        loop {
            let response = hook_gate_exchange(&socket, &hook_gate_request()).await;
            if response.code != crate::claude_hooks::CLAUDE_HOOK_NOT_READY {
                break response;
            }
            assert_hook_gate_rejection(&response, crate::claude_hooks::CLAUDE_HOOK_NOT_READY);
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_hook_gate_rejection(&response, crate::claude_hooks::CLAUDE_HOOK_REJECTED);
    abort_and_drain_tasks(&mut tasks).await;
}

#[tokio::test(flavor = "current_thread")]
async fn abandoned_runtime_activation_closes_the_queued_hook_connection() {
    let fixture = Fixture::new();
    let server = hook_pump_server(&fixture).await;
    let socket = server.environment().socket_path().to_path_buf();
    let mut client = crate::local_unix::LocalUnixStream::connect(socket)
        .await
        .unwrap();
    let mut tasks = JoinSet::new();
    let mut pump = claude_hook::spawn_admission_pump(
        &mut tasks,
        server,
        Arc::clone(&fixture.state),
        hook_pump_shell(&fixture),
    );
    pump.wait_ready().await.unwrap();

    drop(pump);
    let response = tokio::time::timeout(
        HOOK_GATE_SETTLEMENT_TIMEOUT,
        crate::claude_hooks::read_frame::<_, crate::claude_hooks::ClaudeNativeResponse>(
            &mut client,
        ),
    )
    .await
    .unwrap();
    assert!(response.is_err());
    let completion = tokio::time::timeout(HOOK_GATE_SETTLEMENT_TIMEOUT, tasks.join_next())
        .await
        .unwrap();
    assert!(matches!(completion, Some(Ok(Some(_)))));
    assert!(tasks.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn hook_pump_cannot_consume_the_runtime_shutdown_notification() {
    let fixture = Fixture::new();
    let server = hook_pump_server(&fixture).await;
    let mut tasks = JoinSet::new();
    let mut pump = claude_hook::spawn_admission_pump(
        &mut tasks,
        server,
        Arc::clone(&fixture.state),
        hook_pump_shell(&fixture),
    );
    pump.wait_ready().await.unwrap();
    pump.activate().unwrap();
    tokio::task::yield_now().await;
    let mut shutdown = Box::pin(fixture.state.shutdown.notified());
    assert!(hook_pump_poll_once(shutdown.as_mut()).await.is_pending());

    fixture.state.shutdown.notify_one();
    tokio::time::timeout(HOOK_GATE_SETTLEMENT_TIMEOUT, shutdown)
        .await
        .unwrap();
    {
        let mut completion = Box::pin(tasks.join_next());
        assert!(hook_pump_poll_once(completion.as_mut()).await.is_pending());
    }
    abort_and_drain_tasks(&mut tasks).await;
}

pub(super) fn hook_gate_request() -> crate::claude_hooks::ClaudeNativeRequest {
    crate::claude_hooks::ClaudeNativeRequest {
        schema_version: crate::claude_hooks::CLAUDE_NATIVE_SCHEMA_VERSION,
        scope: crate::claude_hooks::ClaudeNativeScope {
            terminal_id: Uuid::new_v4(),
            runtime_epoch: Uuid::new_v4(),
            agent: crate::claude_hooks::ClaudeProcessIdentity {
                pid: HOOK_GATE_AGENT_PID,
                start_time: HOOK_GATE_AGENT_START,
            },
        },
        event_sequence: crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
        event: crate::claude_hooks::ClaudeLifecycleEvent::SessionStart {
            event_id: Uuid::new_v4(),
            provider_session_id: HOOK_GATE_AGENT_SESSION.to_owned(),
            source: crate::claude_hooks::ClaudeSessionStartSource::Fresh,
            state: crate::claude_hooks::ClaudeHookState::WaitingForInput,
            cwd: std::path::PathBuf::from(HOOK_GATE_PROJECT),
            transcript_path: std::path::Path::new(HOOK_GATE_PROJECT)
                .join(HOOK_GATE_TRANSCRIPT_FILE),
        },
    }
}
