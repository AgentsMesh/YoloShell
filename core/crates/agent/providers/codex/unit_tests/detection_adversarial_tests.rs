use super::test_support::Fixture;
use super::{constants, test_support};
use crate::agent::{AgentRecoveryAvailability, AgentState, DetectionResult};
use serde_json::json;

const SUBAGENT_KEY: &str = "subagent";
const SUBAGENT_VALUE: &str = "thread";
const NODE_ENTRYPOINT: &str = "/opt/node_modules/@openai/codex/bin/codex.js";

#[test]
fn pid_reuse_cannot_inherit_or_replace_the_old_binding() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    let initial = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[],
        Some(&[test_support::SESSION_ID]),
    );
    let mut detector = fixture.detector();
    assert!(matches!(
        detector.detect(&[initial]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.state == AgentState::Unknown
                && binding.recovery_availability
                    == AgentRecoveryAvailability::UnmanagedLaunch
    ));

    fixture.write_main(test_support::NEW_SESSION_ID);
    let reused = fixture.process(
        test_support::PROCESS_PID,
        test_support::OTHER_PROCESS_START_TIME,
        &[],
        Some(&[test_support::NEW_SESSION_ID]),
    );
    assert!(matches!(
        detector.detect(&[reused]).unwrap(),
        DetectionResult::StatusChanged(binding)
            if binding.provider_session_id == test_support::SESSION_ID
                && binding.state == AgentState::Unknown
                && binding.recovery_availability == AgentRecoveryAvailability::Unavailable
                && binding.process_pid == Some(test_support::PROCESS_PID)
                && binding.process_start_time == Some(test_support::PROCESS_START_TIME)
    ));
}

#[test]
fn same_cwd_terminals_remain_scoped_to_their_associated_rollouts() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    fixture.write_main(test_support::OTHER_SESSION_ID);
    let left = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[],
        Some(&[test_support::SESSION_ID]),
    );
    let right = fixture.process(
        test_support::OTHER_PROCESS_PID,
        test_support::OTHER_PROCESS_START_TIME,
        &[],
        Some(&[test_support::OTHER_SESSION_ID]),
    );
    let mut left_detector = fixture.detector();
    let mut right_detector = fixture.detector();
    assert!(matches!(
        left_detector.detect(&[left]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == test_support::SESSION_ID
    ));
    assert!(matches!(
        right_detector.detect(&[right]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == test_support::OTHER_SESSION_ID
    ));
}

#[test]
fn subagent_churn_cannot_replace_the_unique_main_rollout() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    let subagent = fixture.evidence(test_support::OTHER_SESSION_ID);
    fixture.write_values(
        &subagent,
        &[fixture.metadata(
            test_support::OTHER_SESSION_ID,
            json!({(SUBAGENT_KEY): SUBAGENT_VALUE}),
        )],
    );
    let process = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[constants::RESUME_ARGUMENT, test_support::SESSION_ID],
        Some(&[test_support::SESSION_ID, test_support::OTHER_SESSION_ID]),
    );
    assert!(matches!(
        fixture.detector().detect(&[process]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == test_support::SESSION_ID
    ));
}

#[test]
fn multiple_main_rollouts_cannot_override_hook_owned_uncertainty() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    let initial = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[],
        Some(&[test_support::SESSION_ID]),
    );
    let mut detector = fixture.detector();
    assert!(matches!(
        detector.detect(&[initial]).unwrap(),
        DetectionResult::BindingChanged(_)
    ));
    fixture.write_main(test_support::NEW_SESSION_ID);
    let ambiguous = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[],
        Some(&[test_support::SESSION_ID, test_support::NEW_SESSION_ID]),
    );
    assert_eq!(
        detector.detect(&[ambiguous]).unwrap(),
        DetectionResult::Unchanged
    );
}

#[test]
fn npm_launcher_cannot_ambiguous_the_native_process_evidence() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    let arguments = [constants::RESUME_ARGUMENT, test_support::SESSION_ID];
    let mut launcher = fixture.process(
        test_support::OTHER_PROCESS_PID,
        test_support::OTHER_PROCESS_START_TIME,
        &arguments,
        Some(&[]),
    );
    launcher.executable = constants::NODE_EXECUTABLE.into();
    launcher.argv = [constants::NODE_EXECUTABLE, NODE_ENTRYPOINT]
        .into_iter()
        .chain(arguments)
        .map(str::to_owned)
        .collect();
    let mut native = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &arguments,
        Some(&[test_support::SESSION_ID]),
    );
    native.parent_pid = launcher.pid;

    assert!(matches!(
        fixture.detector().detect(&[launcher, native]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == test_support::SESSION_ID
                && binding.process_pid == Some(test_support::PROCESS_PID)
    ));
}
