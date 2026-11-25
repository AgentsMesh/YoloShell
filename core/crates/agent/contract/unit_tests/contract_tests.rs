use super::constants::{CODEX_EXECUTABLE, RESUME_ARGUMENT};
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;
use yolo_agent_contract::{
    AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState, CLAUDE_CODE_KIND, CODEX_KIND,
    DetectionResult, ObservedProcess, ResumePlan,
};

const SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const EXECUTABLE: &str = CODEX_EXECUTABLE;
const CWD: &str = "/workspace";
const FUTURE_KIND: &str = "future-agent";

#[test]
fn live_execution_and_known_activity_are_separate_facts() {
    for (state, active) in [
        (AgentState::Active, true),
        (AgentState::Running, true),
        (AgentState::WaitingForInput, true),
        (AgentState::WaitingForPermission, true),
        (AgentState::Exited, false),
        (AgentState::Unknown, false),
    ] {
        assert_eq!(state.is_active(), active);
        let encoded = serde_json::to_vec(&state).unwrap();
        assert_eq!(
            serde_json::from_slice::<AgentState>(&encoded).unwrap(),
            state
        );
    }
}

#[test]
fn provider_kinds_use_stable_wire_names() {
    assert_eq!(AgentKind::Codex.as_str(), CODEX_KIND);
    assert_eq!(AgentKind::ClaudeCode.as_str(), CLAUDE_CODE_KIND);
    let future = AgentKind::from_name(FUTURE_KIND);
    assert_eq!(future.as_str(), FUTURE_KIND);
    let encoded = serde_json::to_string(&future).unwrap();
    assert_eq!(serde_json::from_str::<AgentKind>(&encoded).unwrap(), future);
}

#[test]
fn process_identity_requires_nonzero_start_time() {
    let process = ObservedProcess {
        pid: 10,
        start_time: 20,
        parent_pid: 1,
        process_group: 7,
        foreground_group: 7,
        executable: EXECUTABLE.to_owned(),
        argv: vec![EXECUTABLE.to_owned()],
        environment: None,
        cwd: Some(PathBuf::from(CWD)),
        evidence_paths: None,
    };
    assert!(process.is_foreground());
    assert!(process.identity_matches(10, 20));
    assert!(!process.identity_matches(10, 0));
    assert!(!process.identity_matches(11, 20));
}

#[test]
fn binding_round_trips_with_canonical_uuid() {
    let session_id = Uuid::parse_str(SESSION_ID).unwrap();
    let binding = AgentBinding {
        schema_version: 5,
        terminal_id: session_id,
        kind: AgentKind::Codex,
        provider_session_id: SESSION_ID.to_owned(),
        executable: EXECUTABLE.to_owned(),
        cwd: CWD.to_owned(),
        evidence_path: "/workspace/evidence.jsonl".to_owned(),
        data_home: "/home/.codex".to_owned(),
        process_pid: Some(10),
        process_start_time: Some(20),
        state: AgentState::Running,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: 1,
        updated_at_ms: 2,
    };
    let encoded = serde_json::to_vec(&binding).unwrap();
    assert_eq!(
        serde_json::from_slice::<AgentBinding>(&encoded).unwrap(),
        binding
    );
}

#[test]
fn contract_sum_types_preserve_payloads() {
    let plan = ResumePlan {
        executable: EXECUTABLE.to_owned(),
        args: vec![RESUME_ARGUMENT.to_owned()],
        environment: BTreeMap::new(),
    };
    assert_eq!(plan.args, vec![RESUME_ARGUMENT]);
    assert_eq!(DetectionResult::Unchanged, DetectionResult::Unchanged);
}
