use super::constants;
use super::test_support::{self, Fixture};
use crate::agent::{AgentRecoveryAvailability, AgentState, DetectionResult};

#[test]
fn fresh_process_binds_only_one_associated_main_rollout() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    let process = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[],
        Some(&[test_support::SESSION_ID]),
    );
    assert!(matches!(
        fixture.detector().detect(&[process]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == test_support::SESSION_ID
    ));

    let unassociated = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[],
        None,
    );
    assert_eq!(
        fixture.detector().detect(&[unassociated]).unwrap(),
        DetectionResult::Unchanged
    );

    fixture.write_main(test_support::OTHER_SESSION_ID);
    let ambiguous = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[],
        Some(&[test_support::SESSION_ID, test_support::OTHER_SESSION_ID]),
    );
    assert_eq!(
        fixture.detector().detect(&[ambiguous]).unwrap(),
        DetectionResult::Unchanged
    );
}

#[test]
fn initial_resume_requires_associated_generation_to_match_the_launch_hint() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    fixture.write_main(test_support::OTHER_SESSION_ID);
    let arguments = [constants::RESUME_ARGUMENT, test_support::SESSION_ID];
    let matching = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &arguments,
        None,
    );
    assert!(matches!(
        fixture.detector().detect(&[matching]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == test_support::SESSION_ID
    ));

    let mismatch = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &arguments,
        Some(&[test_support::OTHER_SESSION_ID]),
    );
    assert_eq!(
        fixture.detector().detect(&[mismatch]).unwrap(),
        DetectionResult::Unchanged
    );
}

#[test]
fn same_process_can_advance_after_codex_new_creates_a_rollout() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    let arguments = [constants::RESUME_ARGUMENT, test_support::SESSION_ID];
    let initial = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &arguments,
        Some(&[test_support::SESSION_ID]),
    );
    let mut detector = fixture.detector();
    assert!(matches!(
        detector.detect(&[initial]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == test_support::SESSION_ID
    ));

    fixture.write_main(test_support::NEW_SESSION_ID);
    let after_new = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &arguments,
        Some(&[test_support::NEW_SESSION_ID]),
    );
    assert!(matches!(
        detector.detect(&[after_new]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == test_support::NEW_SESSION_ID
                && binding.state == AgentState::Unknown
                && binding.recovery_availability
                    == AgentRecoveryAvailability::UnmanagedLaunch
    ));
}

#[test]
fn resume_discovery_filters_rollouts_by_filename_before_inspection() {
    let fixture = Fixture::new();
    let expected = fixture.write_main(test_support::SESSION_ID);
    fixture.write_main(test_support::OTHER_SESSION_ID);
    let process = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[constants::RESUME_ARGUMENT, test_support::SESSION_ID],
        None,
    );
    assert_eq!(
        fixture
            .detector()
            .paths_for_process(&crate::agent::AgentKind::Codex, &process)
            .unwrap(),
        [expected]
    );
}
