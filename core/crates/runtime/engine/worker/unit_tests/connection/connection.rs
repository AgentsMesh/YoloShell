use crate::worker::tests::*;
pub(super) async fn attach_worker(client: &mut WorkerConnection, fixture: &Fixture) {
    let attach = RequestEnvelope::new(
        "attach-request",
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: fixture.terminal_id(),
            after_seq: 0,
        })
        .unwrap(),
    );
    let response = client.exchange(&attach).await.unwrap();
    assert!(response.ok);
    let attached: AttachResult = response.decode_payload().unwrap();
    assert_eq!(attached.runtime_epoch, fixture.state.runtime_epoch());
}

#[tokio::test]
async fn worker_auth_fences_stale_controllers_and_malformed_payloads() {
    let fixture = Fixture::new();
    fixture
        .state
        .controller_generation
        .store(3, Ordering::Release);
    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });
    let stale = RequestEnvelope::new(
        "stale",
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: "test-token".into(),
            controller_generation: 2,
        })
        .unwrap(),
    );
    write_frame(&mut client, &stale).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    assert_eq!(response.error.unwrap().code, "conflict");
    task.await.unwrap().unwrap();

    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });
    let malformed = RequestEnvelope::new("bad", WORKER_AUTH_METHOD, yolo_capnp_value::Value::Null);
    write_frame(&mut client, &malformed).await.unwrap();
    assert!(task.await.unwrap().is_err());
}

#[tokio::test]
async fn attached_worker_streams_all_event_variants_and_filters_barrier_output() {
    use crate::agent::{AgentKind, AgentState};

    let fixture = Fixture::new();
    let (mut client, task) = authenticated_worker_connection(&fixture, 1).await;
    attach_worker(&mut client, &fixture).await;

    let stale = OutputChunk {
        runtime_epoch: fixture.state.runtime_epoch(),
        sequence: 0,
        data: b"stale".to_vec(),
    };
    let live = OutputChunk {
        sequence: 1,
        data: b"live".to_vec(),
        ..stale.clone()
    };
    fixture
        .state
        .events
        .send(WorkerEvent::Output(stale))
        .unwrap();
    fixture
        .state
        .events
        .send(WorkerEvent::Output(live))
        .unwrap();
    let output = client.next_event().await.unwrap();
    assert_eq!(output.event, event::TERMINAL_OUTPUT);

    fixture
        .state
        .events
        .send(WorkerEvent::WriterLease {
            event_type: event::WRITER_LEASE_ACQUIRED,
            payload: serde_json::json!({"generation": 1}),
        })
        .unwrap();
    let lease = client.next_event().await.unwrap();
    assert_eq!(lease.event, event::WRITER_LEASE_ACQUIRED);

    fixture
        .state
        .events
        .send(WorkerEvent::Agent {
            event_type: event::AGENT_BINDING_CHANGED,
            binding: AgentBinding {
                schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
                terminal_id: fixture.terminal_id(),
                kind: AgentKind::Codex,
                provider_session_id: "agent".into(),
                executable: crate::agent::codex::constants::EXECUTABLE.into(),
                cwd: "/tmp".into(),
                evidence_path: "/tmp/evidence.jsonl".into(),
                data_home: "/tmp/.codex".into(),
                process_pid: Some(2),
                process_start_time: Some(2),
                state: AgentState::Running,
                recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
                detected_at_ms: 1,
                updated_at_ms: 2,
            },
        })
        .unwrap();
    let agent = client.next_event().await.unwrap();
    assert_eq!(agent.event, event::AGENT_BINDING_CHANGED);

    fixture
        .state
        .events
        .send(WorkerEvent::Exited(ExitRecord {
            runtime_epoch: fixture.state.runtime_epoch(),
            code: Some(0),
            signal: None,
            reason: ExitReason::Natural,
            observed_at_ms: now_ms(),
            last_output_sequence: 1,
        }))
        .unwrap();
    let exited = client.next_event().await.unwrap();
    assert_eq!(exited.event, event::SESSION_EXITED);
    drop(client);
    let _ = task.await.unwrap();
}

#[tokio::test]
async fn attachment_rejects_wrong_session_then_reports_malformed_payload() {
    let fixture = Fixture::new();
    let (mut client, task) = authenticated_worker_connection(&fixture, 1).await;
    let wrong = RequestEnvelope::new(
        "wrong",
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: Uuid::new_v4(),
            after_seq: 0,
        })
        .unwrap(),
    );
    let response = client.exchange(&wrong).await.unwrap();
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::INVALID_REQUEST
    );

    let malformed = RequestEnvelope::new(
        "malformed",
        method::ATTACH_SESSION,
        yolo_capnp_value::Value::Null,
    );
    let response = client.exchange(&malformed).await.unwrap();
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::INVALID_REQUEST
    );
    drop(client);
    let _ = task.await.unwrap();
}

