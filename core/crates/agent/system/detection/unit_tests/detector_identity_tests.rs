use super::*;
#[path = "detector_identity_support_tests.rs"]
mod support;
use support::*;

#[test]
fn pid_reuse_exits_before_an_inspected_candidate_can_be_applied() {
    let fixture = Fixture::new();
    let result = fixture
        .detector()
        .detect(&[fixture.process(REUSED_START, Some(vec![fixture.original.clone()]))])
        .unwrap();
    assert!(matches!(
        result,
        DetectionResult::StatusChanged(binding)
            if binding.state == AgentState::Exited
                && binding.process_pid.is_none()
                && binding.process_start_time.is_none()
    ));
    assert_eq!(
        *fixture.inspected.lock().unwrap(),
        [fixture.original.clone()]
    );
}

#[test]
fn pid_reuse_requires_an_exit_transition_before_binding_new_evidence() {
    let fixture = Fixture::new();
    let mut detector = fixture.detector();
    let process = fixture.process(REUSED_START, Some(vec![fixture.reused.clone()]));
    let exited = detector.detect(std::slice::from_ref(&process)).unwrap();
    assert!(matches!(
        exited,
        DetectionResult::StatusChanged(binding) if binding.state == AgentState::Exited
    ));
    let result = detector.detect(&[process]).unwrap();
    assert!(matches!(
        result,
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == REUSED_SESSION
                && binding.process_start_time == Some(REUSED_START)
    ));
    assert_eq!(
        *fixture.inspected.lock().unwrap(),
        [fixture.reused.clone(), fixture.reused.clone()]
    );

    let latest_exit = detector
        .detect(&[fixture.process(LATEST_START, None)])
        .unwrap();
    assert!(
        matches!(latest_exit, DetectionResult::StatusChanged(binding) if binding.state == AgentState::Exited)
    );
    assert_eq!(
        detector
            .detect(&[fixture.process(LATEST_START + 1, None)])
            .unwrap(),
        DetectionResult::Unchanged
    );
    assert_eq!(
        *fixture.inspected.lock().unwrap(),
        [fixture.reused.clone(), fixture.reused.clone()]
    );
}

#[test]
fn stable_process_identity_refreshes_the_existing_evidence() {
    let fixture = Fixture::new();
    let result = fixture
        .detector()
        .detect(&[fixture.process(ORIGINAL_START, Some(Vec::new()))])
        .unwrap();
    assert!(matches!(
        result,
        DetectionResult::StatusChanged(binding)
            if binding.provider_session_id == ORIGINAL_SESSION
                && binding.process_start_time == Some(ORIGINAL_START)
                && binding.state == AgentState::WaitingForInput
    ));
    assert_eq!(
        *fixture.inspected.lock().unwrap(),
        [fixture.original.clone()]
    );
}

#[test]
fn same_process_empty_association_ignores_a_new_unassociated_transcript() {
    let fixture = Fixture::new();
    let mut detector = fixture.detector();
    fixture.refresh_reused_evidence();
    let result = detector
        .detect(&[fixture.process(ORIGINAL_START, Some(Vec::new()))])
        .unwrap();
    assert!(matches!(
        result,
        DetectionResult::StatusChanged(binding)
            if binding.provider_session_id == ORIGINAL_SESSION
                && binding.process_start_time == Some(ORIGINAL_START)
                && binding.state == AgentState::WaitingForInput
    ));
    assert_eq!(
        *fixture.inspected.lock().unwrap(),
        [fixture.original.clone()]
    );
}
