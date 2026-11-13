use std::time::Duration;

#[test]
fn exact_positive_publishes_before_the_minimum_poll_work() {
    let exact = candidate(THIRD_PID, THIRD_GENERATION, false);
    let ambient = candidate(FIRST_PID, FIRST_GENERATION, false);
    let priorities = HashSet::from([exact.live]);
    let mut evaluated = Vec::new();

    let result = advance::advance_with_priority_boundary(
        candidates([ambient, exact]),
        &ReleasedScanState::default(),
        boundary::ReleasedPollBoundary::with_time_slice(
            MAX_PTY_PROOF_CANDIDATES,
            boundary::RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL,
            Duration::ZERO,
        ),
        ReleasedSweepPolicy::Frozen,
        &priorities,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(candidate == exact))
        },
    )
    .unwrap();

    assert_eq!(evaluated, [exact]);
    assert_eq!(result.scheduling_status, CapabilityScanStatus::Pending);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(
        result.candidates.observed,
        HashSet::from([exact.live.lineage_id()])
    );
    assert_eq!(
        result.advance.next.released.pending_lineages_for_test(),
        [ambient.live.lineage_id()]
    );
}
