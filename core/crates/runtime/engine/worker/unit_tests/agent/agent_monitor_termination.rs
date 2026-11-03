
#[tokio::test]
async fn ended_shell_retries_until_final_agent_state_is_persisted() {
    let fixture = Fixture::new();
    fixture.state.child_waited.store(true, Ordering::Release);
    let binding_path = fixture.state.paths.agent_binding(fixture.terminal_id());
    std::fs::create_dir_all(&binding_path).unwrap();
    let (registry, provider_runtime) = monitor_runtime();
    let config = AgentMonitorConfig {
        registry,
        provider_runtime,
        context: binding_context(&fixture),
        binding_path: binding_path.clone(),
    };
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(supervise_agent_monitor(state, config, u32::MAX));
    tokio::time::sleep(AGENT_MONITOR_REPAIR_DELAY).await;
    std::fs::remove_dir(&binding_path).unwrap();
    let expected = binding(&fixture, crate::agent::AgentState::Running);
    AgentBindingStore::new(binding_path.clone())
        .save(&expected)
        .unwrap();

    tokio::time::timeout(AGENT_MONITOR_REPAIR_TIMEOUT, task)
        .await
        .unwrap()
        .unwrap();
    assert_exact_binding_and_fail_closed_recovery(
        &AgentBindingStore::new(binding_path),
        &binding_context(&fixture),
        &expected,
    );
}
const AGENT_MONITOR_REPAIR_DELAY: Duration = Duration::from_millis(20);
const AGENT_MONITOR_REPAIR_TIMEOUT: Duration = Duration::from_secs(2);

include!("../../test_support/unit/agent/agent_monitor_state_assertions.rs");
include!("../session/session_removal_tests.rs");
