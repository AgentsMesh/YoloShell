use super::*;
use crate::agent::constants::{BINDING_SCHEMA_VERSION, MISSED_DETECTION_THRESHOLD};
use crate::agent::{AgentRecoveryAvailability, AgentState, DetectionResult, ObservedProcess};
use std::path::PathBuf;
use uuid::Uuid;

const TEST_CWD: &str = "/workspace";
const TEST_DATA_HOME: &str = "/workspace/agent-home";
const TEST_EVIDENCE: &str = "/workspace/agent-home/session.jsonl";
const TEST_SESSION: &str = "550e8400-e29b-41d4-a716-446655440000";
const TEST_PID: u32 = 42_101;
const ORIGINAL_START: u64 = 52_101;
const REUSED_START: u64 = 52_102;
const PROCESS_GROUP: i32 = 62_101;
const FIRST_MISS: u8 = 1;

#[test]
fn codex_pid_reuse_fails_closed_without_a_final_hook() {
    assert_pid_reuse_is_unconfirmed(AgentKind::Codex);
}

#[test]
fn claude_pid_reuse_fails_closed_without_a_final_hook() {
    assert_pid_reuse_is_unconfirmed(AgentKind::ClaudeCode);
}

#[test]
fn an_unmanaged_hook_binding_cannot_be_replaced_after_pid_reuse() {
    for kind in [AgentKind::Codex, AgentKind::ClaudeCode] {
        let mut detector = detector(kind.clone(), AgentState::Unknown);
        detector.binding.as_mut().unwrap().recovery_availability =
            AgentRecoveryAvailability::UnmanagedLaunch;
        let result = detector.detect(&[process(&kind, REUSED_START)]).unwrap();
        assert!(matches!(
            result,
            DetectionResult::StatusChanged(value)
                if value.state == AgentState::Unknown
                    && value.recovery_availability
                        == AgentRecoveryAvailability::Unavailable
        ));
    }
}

#[test]
fn codex_missed_scans_fail_closed_without_a_final_hook() {
    assert_missed_scans_are_unconfirmed(AgentKind::Codex);
}

#[test]
fn claude_missed_scans_fail_closed_without_a_final_hook() {
    assert_missed_scans_are_unconfirmed(AgentKind::ClaudeCode);
}

#[test]
fn provider_child_wait_does_not_claim_a_clean_codex_exit() {
    assert_session_exit_is_unconfirmed(AgentKind::Codex);
}

#[test]
fn provider_child_wait_does_not_claim_a_clean_claude_exit() {
    assert_session_exit_is_unconfirmed(AgentKind::ClaudeCode);
}

#[test]
fn authoritative_only_child_wait_retains_clean_exit_semantics() {
    let context = DetectionContext::new(Uuid::new_v4(), TEST_CWD.into(), []);
    let binding = binding(AgentKind::Codex, AgentState::Running);
    let mut detector = AgentDetector::new(
        context,
        AgentRegistry::from_adapters(Vec::new()).unwrap(),
        Some(binding),
    )
    .unwrap();
    assert!(matches!(
        detector.session_exited(),
        DetectionResult::StatusChanged(value)
            if value.state == AgentState::Exited
                && value.recovery_availability == AgentRecoveryAvailability::Available
                && value.process_pid.is_none()
                && value.process_start_time.is_none()
    ));
}

#[test]
fn an_already_finalized_hook_binding_is_not_downgraded() {
    for kind in [AgentKind::Codex, AgentKind::ClaudeCode] {
        let context = DetectionContext::new(Uuid::new_v4(), TEST_CWD.into(), []);
        let mut finalized = binding(kind, AgentState::Exited);
        finalized.recovery_availability = AgentRecoveryAvailability::Available;
        let mut detector =
            AgentDetector::new(context, yolo_agent_providers::builtin_agent_registry(), Some(finalized)).unwrap();
        assert_eq!(detector.session_exited(), DetectionResult::Unchanged);
    }
}

fn assert_pid_reuse_is_unconfirmed(kind: AgentKind) {
    let mut detector = detector(kind.clone(), AgentState::Running);
    let reused = process(&kind, REUSED_START);
    let result = detector.detect(&[reused.clone()]).unwrap();
    assert!(matches!(
        result,
        DetectionResult::StatusChanged(value)
            if value.state == AgentState::Unknown
                && value.recovery_availability == AgentRecoveryAvailability::Unavailable
                && value.process_pid == Some(TEST_PID)
                && value.process_start_time == Some(ORIGINAL_START)
    ));
    assert_eq!(
        detector.detect(&[reused]).unwrap(),
        DetectionResult::Unchanged
    );
}

fn assert_missed_scans_are_unconfirmed(kind: AgentKind) {
    let mut detector = detector(kind, AgentState::Running);
    for _ in FIRST_MISS..MISSED_DETECTION_THRESHOLD {
        assert_eq!(detector.detect(&[]).unwrap(), DetectionResult::Unchanged);
    }
    let result = detector.detect(&[]).unwrap();
    assert!(matches!(
        result,
        DetectionResult::StatusChanged(value)
            if value.state == AgentState::Unknown
                && value.recovery_availability == AgentRecoveryAvailability::Unavailable
                && value.process_pid == Some(TEST_PID)
                && value.process_start_time == Some(ORIGINAL_START)
    ));
}

fn assert_session_exit_is_unconfirmed(kind: AgentKind) {
    let mut detector = detector(kind, AgentState::Running);
    let result = detector.session_exited();
    assert!(matches!(
        result,
        DetectionResult::StatusChanged(value)
            if value.state == AgentState::Unknown
                && value.recovery_availability == AgentRecoveryAvailability::Unavailable
                && value.process_pid == Some(TEST_PID)
                && value.process_start_time == Some(ORIGINAL_START)
    ));
}

fn detector(kind: AgentKind, state: AgentState) -> AgentDetector {
    let context = DetectionContext::new(Uuid::new_v4(), TEST_CWD.into(), []);
    AgentDetector::new(
        context,
        yolo_agent_providers::builtin_agent_registry(),
        Some(binding(kind, state)),
    )
    .unwrap()
}

fn binding(kind: AgentKind, state: AgentState) -> AgentBinding {
    AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind: kind.clone(),
        provider_session_id: TEST_SESSION.into(),
        executable: executable(&kind).into(),
        cwd: TEST_CWD.into(),
        evidence_path: TEST_EVIDENCE.into(),
        data_home: TEST_DATA_HOME.into(),
        process_pid: (state != AgentState::Exited).then_some(TEST_PID),
        process_start_time: (state != AgentState::Exited).then_some(ORIGINAL_START),
        state,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: i64::default(),
        updated_at_ms: i64::default(),
    }
}

fn process(kind: &AgentKind, start_time: u64) -> ObservedProcess {
    let executable = executable(kind).to_owned();
    ObservedProcess {
        pid: TEST_PID,
        start_time,
        parent_pid: crate::agent::constants::MIN_PROCESS_PID,
        process_group: PROCESS_GROUP,
        foreground_group: PROCESS_GROUP,
        executable: executable.clone(),
        argv: vec![executable],
        environment: None,
        cwd: Some(PathBuf::from(TEST_CWD)),
        evidence_paths: None,
    }
}

fn executable(kind: &AgentKind) -> &'static str {
    match kind {
        AgentKind::Codex => crate::agent::codex::constants::EXECUTABLE,
        AgentKind::ClaudeCode => crate::agent::claude::constants::EXECUTABLE,
        AgentKind::Other(_) => unreachable!(),
    }
}
