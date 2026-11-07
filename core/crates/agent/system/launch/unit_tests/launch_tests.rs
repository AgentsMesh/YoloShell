use super::*;
use uuid::Uuid;
use yolo_agent_contract::{AgentKind, AgentRecoveryAvailability, AgentState};
use yolo_agent_core::constants::BINDING_SCHEMA_VERSION;
use yolo_application_support::constants::WORKER_LAUNCH_SCHEMA_VERSION;

fn launch() -> WorkerLaunchSpec {
    WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        socket_path: "/tmp/worker.sock".into(),
        auth_token: "token".into(),
        controller_generation: 1,
        command: "/bin/sh".into(),
        args: Vec::new(),
        cwd: "/tmp".into(),
        environment: Default::default(),
        initial_input: None,
        cols: 80,
        rows: 24,
        context: None,
    }
}

fn registry() -> AgentRegistry {
    let runtime = yolo_agent_providers::builtin_provider_runtime();
    yolo_agent_providers::agent_registry_from_runtime(&runtime)
}

fn binding(kind: AgentKind, state: AgentState, data_home: &str) -> AgentBinding {
    AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind,
        provider_session_id: "agent-session".into(),
        executable: "agent".into(),
        cwd: "/tmp".into(),
        evidence_path: "/tmp/evidence.jsonl".into(),
        data_home: data_home.into(),
        process_pid: (state != AgentState::Exited).then_some(1),
        process_start_time: (state != AgentState::Exited).then_some(1),
        state,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: 1,
        updated_at_ms: 2,
    }
}

#[test]
fn shell_command_and_resume_cover_both_agent_homes() {
    let shell = shell_command(Path::new("/tmp"));
    assert_eq!(shell.cwd, Path::new("/tmp"));
    assert_eq!(shell.args, ["-l"]);
    assert!(!shell.executable.as_os_str().is_empty());

    let mut codex = launch();
    apply_resume_with_registry(
        &mut codex,
        &binding(AgentKind::Codex, AgentState::Exited, "/codex-home"),
        ResumeIntent::Explicit,
        &registry(),
    )
    .unwrap();
    assert_eq!(
        codex
            .environment
            .get(yolo_codex::codex::constants::DATA_HOME_ENV),
        Some(&"/codex-home".to_owned())
    );
    assert!(codex.initial_input.unwrap().ends_with(b"agent-session\n"));

    let mut claude = launch();
    apply_resume_with_registry(
        &mut claude,
        &binding(AgentKind::ClaudeCode, AgentState::Exited, ""),
        ResumeIntent::Explicit,
        &registry(),
    )
    .unwrap();
    assert!(claude.environment.is_empty());
    assert!(claude.initial_input.unwrap().ends_with(b"agent-session\n"));
}

#[test]
fn unsupported_registry_fails_without_mutating_launch() {
    let mut spec = launch();
    let registry = yolo_agent_providers::AgentRegistry::from_adapters(Vec::new()).unwrap();
    let error = apply_resume_with_registry(
        &mut spec,
        &binding(AgentKind::Codex, AgentState::Running, "/codex-home"),
        ResumeIntent::Automatic,
        &registry,
    )
    .unwrap_err();
    assert!(error.to_string().contains("unsupported agent kind"));
    assert_eq!(spec.initial_input, None);
    assert!(spec.environment.is_empty());
}

#[test]
fn resume_intent_policy_covers_the_agent_state_matrix() {
    for state in [
        AgentState::Active,
        AgentState::Running,
        AgentState::WaitingForInput,
        AgentState::WaitingForPermission,
        AgentState::Exited,
    ] {
        let binding = binding(AgentKind::Codex, state, "/codex-home");
        assert_eq!(
            ResumeIntent::Explicit.policy(&binding),
            ResumePolicy::Provider
        );
    }
    let running = binding(AgentKind::Codex, AgentState::Running, "/codex-home");
    let waiting = binding(AgentKind::Codex, AgentState::WaitingForInput, "/codex-home");
    let permission = binding(
        AgentKind::Codex,
        AgentState::WaitingForPermission,
        "/codex-home",
    );
    let exited = binding(AgentKind::Codex, AgentState::Exited, "/codex-home");
    let unknown = binding(AgentKind::Codex, AgentState::Unknown, "/codex-home");
    let active = binding(AgentKind::Codex, AgentState::Active, "/codex-home");
    assert_eq!(
        ResumeIntent::Automatic.policy(&active),
        ResumePolicy::Provider
    );
    assert_eq!(
        ResumeIntent::Automatic.policy(&running),
        ResumePolicy::Provider
    );
    assert_eq!(
        ResumeIntent::Automatic.policy(&waiting),
        ResumePolicy::Provider
    );
    assert_eq!(
        ResumeIntent::Automatic.policy(&permission),
        ResumePolicy::Provider
    );
    assert_eq!(
        ResumeIntent::Automatic.policy(&exited),
        ResumePolicy::ExplicitOnly
    );
    for intent in [ResumeIntent::Automatic, ResumeIntent::Explicit] {
        assert_eq!(intent.policy(&unknown), ResumePolicy::Provider);
    }
}

#[test]
fn unmanaged_and_unavailable_launches_fail_closed_for_every_resume_intent() {
    for availability in [
        AgentRecoveryAvailability::UnmanagedLaunch,
        AgentRecoveryAvailability::Unavailable,
    ] {
        let mut binding = binding(AgentKind::ClaudeCode, AgentState::Running, "/claude");
        binding.recovery_availability = availability;
        for intent in [ResumeIntent::Automatic, ResumeIntent::Explicit] {
            assert_eq!(intent.policy(&binding), ResumePolicy::FailClosed);
        }
    }
}

#[test]
fn automatic_launch_rejects_non_resumable_agent_states_without_mutation() {
    for state in [AgentState::Exited] {
        let mut spec = launch();
        let error = apply_resume_with_registry(
            &mut spec,
            &binding(AgentKind::Codex, state, "/codex-home"),
            ResumeIntent::Automatic,
            &registry(),
        )
        .unwrap_err();
        assert!(error.to_string().contains(AUTOMATIC_RESUME_STATE_REJECTED));
        assert_eq!(spec.initial_input, None);
        assert!(spec.environment.is_empty());
    }
}
