use super::super::boundary::{
    RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL, RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL,
};
use super::*;
use std::time::Duration;

const EXPIRED_TIME_SLICE: Duration = Duration::ZERO;
const CONSECUTIVE_LIVE_INSPECTIONS: usize = 2;
const CANDIDATE_COUNT: usize = 3;
const REMAINING_CANDIDATE_COUNT: usize = CANDIDATE_COUNT - CONSECUTIVE_LIVE_INSPECTIONS;

#[test]
fn consecutive_live_inspections_consume_one_candidate_each() {
    let current = candidates([
        candidate(FIRST_PID, FIRST_GENERATION, false),
        candidate(SECOND_PID, SECOND_GENERATION, false),
        candidate(THIRD_PID, THIRD_GENERATION, false),
    ]);
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();

    for _ in usize::default()..CONSECUTIVE_LIVE_INSPECTIONS {
        let boundary = super::boundary::ReleasedPollBoundary::with_time_slice(
            CANDIDATE_COUNT,
            RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL,
            EXPIRED_TIME_SLICE,
        );
        let result = advance_with_boundary(
            Arc::clone(&current),
            &state,
            boundary,
            ReleasedSweepPolicy::Live,
            |candidate, _budget| {
                evaluated.push(candidate.live);
                Ok(CandidateEvaluation::Complete(false))
            },
        )
        .unwrap();

        assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
        commit(&mut state, result);
    }

    assert_eq!(evaluated.len(), CONSECUTIVE_LIVE_INSPECTIONS);
    assert_eq!(
        state.pending_lineages_for_test().len(),
        REMAINING_CANDIDATE_COUNT
    );
}

#[test]
fn expired_frozen_quantum_retires_its_protocol_batch() {
    const PID_BASE: u32 = 410_000;
    const GENERATION_BASE: u64 = 420_000;
    const REMAINING_CANDIDATES: usize = 1;
    const TOTAL_CANDIDATES: usize =
        RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL + REMAINING_CANDIDATES;
    let current = (usize::default()..TOTAL_CANDIDATES)
        .map(|offset| {
            candidate(
                PID_BASE + u32::try_from(offset).unwrap(),
                GENERATION_BASE + u64::try_from(offset).unwrap(),
                false,
            )
        })
        .collect::<Vec<_>>()
        .into();
    let mut evaluated = Vec::new();

    let result = advance_with_boundary(
        current,
        &ReleasedScanState::default(),
        super::boundary::ReleasedPollBoundary::with_time_slice(
            TOTAL_CANDIDATES,
            RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL,
            EXPIRED_TIME_SLICE,
        ),
        ReleasedSweepPolicy::Frozen,
        |candidate, _budget| {
            evaluated.push(candidate.live);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(evaluated.len(), RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
}

#[test]
fn frozen_production_makes_progress_within_its_quantum() {
    let current = candidates([
        candidate(FIRST_PID, FIRST_GENERATION, false),
        candidate(SECOND_PID, SECOND_GENERATION, false),
        candidate(THIRD_PID, THIRD_GENERATION, false),
    ]);
    let mut evaluated = Vec::new();

    let result = advance_with_boundary(
        current,
        &ReleasedScanState::default(),
        super::boundary::ReleasedPollBoundary::frozen_production(),
        ReleasedSweepPolicy::Frozen,
        |candidate, _budget| {
            evaluated.push(candidate.live);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert!(!evaluated.is_empty());
    assert!(evaluated.len() <= CANDIDATE_COUNT);
    assert!(matches!(
        result.coverage_status,
        CapabilityScanStatus::Pending | CapabilityScanStatus::Complete
    ));
}
