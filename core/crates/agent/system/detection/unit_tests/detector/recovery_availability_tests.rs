use super::*;
use crate::agent::constants::BINDING_SCHEMA_VERSION;
use crate::agent::launch::{ResumeIntent, ResumePolicy};
use std::path::PathBuf;
use uuid::Uuid;

const AGENT_SESSION_ID: &str = "managed-boundary-session";
const DATA_HOME: &str = "/tmp/yolo-claude-home";
const EVIDENCE_PATH: &str = "/tmp/yolo-claude-home/session.jsonl";
const PROJECT_PATH: &str = "/tmp/yolo-project";
const PROCESS_PID: u32 = 810;
const PROCESS_START_TIME: u64 = 811;
const TIMESTAMP: i64 = 812;
const UNKNOWN_TRANSITION_FAILURE: &str = "unmanaged running binding did not transition to unknown";
const EXIT_TRANSITION_FAILURE: &str = "unmanaged unknown binding did not transition to exited";
const UNKNOWN_BINDING_FAILURE: &str = "binding did not transition to unknown";
const MANAGED_UNKNOWN_FAILURE: &str = "managed binding did not transition to unknown";

#[test]
fn uncertain_activity_does_not_discard_verified_execution_or_resume_evidence() {
    let mut current = binding(AgentRecoveryAvailability::Available);
    current.kind = AgentKind::Codex;
    current.state = AgentState::Active;
    let context = DetectionContext::new(current.terminal_id, PathBuf::from(PROJECT_PATH), []);
    let mut detector =
        AgentDetector::new(context, yolo_agent_providers::builtin_agent_registry(), Some(current.clone())).unwrap();
    assert_eq!(detector.binding.as_ref(), Some(&current));
    assert_eq!(
        detector.synchronize_binding(current.clone()),
        DetectionResult::Unchanged
    );
    assert_eq!(detector.binding.as_ref(), Some(&current));
    for intent in [ResumeIntent::Automatic, ResumeIntent::Explicit] {
        assert_eq!(intent.policy(&current), ResumePolicy::Provider);
    }
}

#[test]
fn unmanaged_availability_survives_unknown_exit_and_resume_policy() {
    let binding = binding(AgentRecoveryAvailability::UnmanagedLaunch);
    let mut detector = detector(binding);
    let DetectionResult::StatusChanged(unknown) = detector.mark_unknown() else {
        panic!("{UNKNOWN_TRANSITION_FAILURE}")
    };
    assert_eq!(
        unknown.recovery_availability,
        AgentRecoveryAvailability::UnmanagedLaunch
    );
    let DetectionResult::StatusChanged(exited) = detector.mark_exited() else {
        panic!("{EXIT_TRANSITION_FAILURE}")
    };
    assert_eq!(
        exited.recovery_availability,
        AgentRecoveryAvailability::UnmanagedLaunch
    );
    for intent in [ResumeIntent::Automatic, ResumeIntent::Explicit] {
        assert_eq!(intent.policy(&exited), ResumePolicy::FailClosed);
    }
}

#[test]
fn managed_unknown_state_is_marked_unavailable() {
    let mut detector = detector(binding(AgentRecoveryAvailability::Available));
    let DetectionResult::StatusChanged(unknown) = detector.mark_unknown() else {
        panic!("{MANAGED_UNKNOWN_FAILURE}")
    };
    assert_eq!(unknown.state, AgentState::Unknown);
    assert_eq!(
        unknown.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
    assert_eq!(detector.mark_unknown(), DetectionResult::Unchanged);
}

#[test]
fn hook_required_session_exit_fails_closed_without_a_final_hook() {
    let mut binding = binding(AgentRecoveryAvailability::Available);
    binding.kind = AgentKind::Codex;
    binding.executable = crate::agent::codex::constants::EXECUTABLE.to_owned();
    let mut detector = detector(binding);
    let DetectionResult::StatusChanged(unknown) = detector.session_exited() else {
        panic!("{UNKNOWN_BINDING_FAILURE}")
    };
    assert_eq!(unknown.state, AgentState::Unknown);
    assert_eq!(
        unknown.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
}

#[test]
fn constructor_normalizes_unknown_binding_and_exit_observation_is_idempotent() {
    let mut binding = binding(AgentRecoveryAvailability::Available);
    binding.kind = AgentKind::Codex;
    binding.executable = crate::agent::codex::constants::EXECUTABLE.to_owned();
    binding.state = AgentState::Unknown;
    let context = DetectionContext::new(binding.terminal_id, PathBuf::from(PROJECT_PATH), []);
    let mut detector =
        AgentDetector::new(context, yolo_agent_providers::builtin_agent_registry(), Some(binding)).unwrap();
    assert_eq!(
        detector.binding.as_ref().unwrap().recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
    assert_eq!(detector.session_exited(), DetectionResult::Unchanged);
    let unknown = detector.binding.as_ref().unwrap();
    assert_eq!(unknown.state, AgentState::Unknown);
    assert_eq!(
        unknown.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
}

#[test]
fn availability_only_synchronization_is_a_status_change() {
    let mut detector = detector(binding(AgentRecoveryAvailability::UnmanagedLaunch));
    let promoted = binding(AgentRecoveryAvailability::Available);
    assert!(matches!(
        detector.synchronize_binding(promoted),
        DetectionResult::StatusChanged(value)
            if value.recovery_availability == AgentRecoveryAvailability::Available
    ));
}

fn detector(binding: AgentBinding) -> AgentDetector {
    AgentDetector {
        context: DetectionContext::new(binding.terminal_id, PathBuf::from(PROJECT_PATH), []),
        registry: yolo_agent_providers::builtin_agent_registry(),
        binding: Some(binding),
        consecutive_misses: 0,
    }
}

fn binding(recovery_availability: AgentRecoveryAvailability) -> AgentBinding {
    AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind: AgentKind::ClaudeCode,
        provider_session_id: AGENT_SESSION_ID.to_owned(),
        executable: crate::agent::claude::constants::EXECUTABLE.to_owned(),
        cwd: PROJECT_PATH.to_owned(),
        evidence_path: EVIDENCE_PATH.to_owned(),
        data_home: DATA_HOME.to_owned(),
        process_pid: Some(PROCESS_PID),
        process_start_time: Some(PROCESS_START_TIME),
        state: AgentState::Running,
        recovery_availability,
        detected_at_ms: TIMESTAMP,
        updated_at_ms: TIMESTAMP,
    }
}
