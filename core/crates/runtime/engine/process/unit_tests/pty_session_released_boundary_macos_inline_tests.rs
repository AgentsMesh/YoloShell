use super::ReleasedPollBoundary;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn live_production_clock_starts_after_round_setup() {
    let boundary = ReleasedPollBoundary::live_production();
    assert!(boundary.started_at.is_none());

    let boundary = boundary.begin_candidate_work();
    assert!(boundary.started_at.is_some());
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn frozen_production_has_a_time_deadline() {
    let boundary = ReleasedPollBoundary::frozen_production().begin_candidate_work();

    assert!(boundary.started_at.is_some());
    assert_eq!(boundary.time_slice, Some(super::RELEASED_SWEEP_TIME_SLICE));
    assert_eq!(
        boundary.minimum_work,
        super::RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL
    );
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn frozen_poll_budget_covers_the_bounded_candidate_universe() {
    assert!(
        super::RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL
            * super::RELEASED_FROZEN_SWEEP_MAX_POLL_COUNT
            >= crate::process::constants::MAX_PTY_PROOF_CANDIDATES
    );
}
