use crate::worker::tests::*;

#[tokio::test]
async fn agent_event_watch_streams_bindings_without_output() {
    use crate::agent::{AgentKind, AgentRecoveryAvailability, AgentState};

    let fixture = Fixture::new();
    let (mut client, task) = authenticated_worker_connection(&fixture, 1).await;
    let watch = RequestEnvelope::new(
        "watch-agent-events",
        method::WATCH_AGENT_EVENTS,
        to_value(&yolo_runtime_protocol::WatchAgentEventsRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: fixture.state.runtime_epoch(),
        })
        .unwrap(),
    );
    let response = client.exchange(&watch).await.unwrap();
    assert!(response.ok);
    // Output is a different plane and must not appear on this stream.
    let _ = fixture
        .state
        .events
        .send(WorkerEvent::Output(OutputChunk {
            runtime_epoch: fixture.state.runtime_epoch(),
            sequence: 1,
            data: b"output-only".to_vec(),
        }));
    let binding = AgentBinding {
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
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: 1,
        updated_at_ms: 2,
    };
    fixture.state.publish_agent_event(
        crate::protocol::event::AGENT_BINDING_CHANGED,
        binding,
    );
    let event = client.next_event().await.unwrap();
    assert_eq!(event.event, crate::protocol::event::AGENT_BINDING_CHANGED);
    drop(client);
    let _ = task.await.unwrap();
}
