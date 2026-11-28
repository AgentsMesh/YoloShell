use super::super::boundary::RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL;
use super::*;
use std::collections::HashSet;
use std::time::Duration;

const LARGE_UNIVERSE_SIZE: usize = 4_096;
const PARTIAL_BATCH_SIZE: usize = 127;
const FIRST_LARGE_PID: u32 = 61_000;
const FIRST_LARGE_GENERATION: u64 = 71_000;
const EXPIRED_TIME_SLICE: Duration = Duration::ZERO;
const TIME_SLICE_CANDIDATE_LIMIT: usize = LARGE_UNIVERSE_SIZE;
const EXPECTED_TIME_SLICE_PROGRESS: usize = 1;
const INCOMPLETE_LARGE_SWEEP_DETAIL: &str =
    "released sweep did not complete within its expected batch count";

#[test]
fn large_universe_completes_across_partial_batches_without_omission() {
    let current = large_universe();
    let mut state = ReleasedScanState::default();
    let mut evaluated = HashSet::new();
    let expected_batches = LARGE_UNIVERSE_SIZE.div_ceil(PARTIAL_BATCH_SIZE);

    for batch in 1..=expected_batches {
        let result = advance_scan(
            Arc::clone(&current),
            &state,
            PARTIAL_BATCH_SIZE,
            |candidate, _budget| {
                assert!(evaluated.insert(candidate.live));
                Ok(CandidateEvaluation::Complete(false))
            },
        )
        .unwrap();
        let complete = result.coverage_status == CapabilityScanStatus::Complete;
        commit(&mut state, result);
        if complete {
            assert_eq!(batch, expected_batches);
            assert_eq!(evaluated.len(), LARGE_UNIVERSE_SIZE);
            return;
        }
        assert!(!state.pending_lineages_for_test().is_empty());
    }
    panic!("{INCOMPLETE_LARGE_SWEEP_DETAIL}");
}

#[test]
fn expired_time_slice_yields_after_progress_without_claiming_completion() {
    let current = candidates([
        candidate(FIRST_PID, FIRST_GENERATION, false),
        candidate(SECOND_PID, SECOND_GENERATION, false),
    ]);
    let boundary = super::boundary::ReleasedPollBoundary::with_time_slice(
        TIME_SLICE_CANDIDATE_LIMIT,
        EXPECTED_TIME_SLICE_PROGRESS,
        EXPIRED_TIME_SLICE,
    );
    let mut evaluated = Vec::new();

    let result = advance_with_boundary(
        current,
        &ReleasedScanState::default(),
        boundary,
        ReleasedSweepPolicy::Live,
        |candidate, _budget| {
            evaluated.push(candidate.live);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(evaluated.len(), EXPECTED_TIME_SLICE_PROGRESS);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
    assert!(result.advance.batch_consumed);
}

#[test]
fn expired_production_slice_completes_the_minimum_work_quantum() {
    let current = large_universe();
    let boundary = super::boundary::ReleasedPollBoundary::with_time_slice(
        TIME_SLICE_CANDIDATE_LIMIT,
        RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL,
        EXPIRED_TIME_SLICE,
    );
    let mut evaluated = Vec::new();

    let result = advance_with_boundary(
        current,
        &ReleasedScanState::default(),
        boundary,
        ReleasedSweepPolicy::Live,
        |candidate, _budget| {
            evaluated.push(candidate.live);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(
        RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL,
        ONE_CANDIDATE_PER_BATCH
    );
    assert_eq!(evaluated.len(), ONE_CANDIDATE_PER_BATCH);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
    assert!(result.advance.batch_consumed);
}

#[test]
fn lazy_tombstones_preserve_the_live_replacement_and_backlog() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let removed = candidate(SECOND_PID, SECOND_GENERATION, false);
    let processed = candidate(THIRD_PID, THIRD_GENERATION, false);
    let replacement = candidate(SECOND_PID, FOURTH_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();

    scan_one(
        candidates([first, removed, processed]),
        &mut state,
        &mut evaluated,
    );
    let replacement_batch = advance_scan(
        candidates([first, replacement]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate.live);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(
        replacement_batch.coverage_status,
        CapabilityScanStatus::Pending
    );
    commit(&mut state, replacement_batch);
    scan_one(candidates([first, replacement]), &mut state, &mut evaluated);

    assert_eq!(evaluated, [processed.live, replacement.live, first.live]);
    assert!(state.pending_lineages_for_test().is_empty());
}

fn scan_one(
    current: Arc<[ReleasedCandidate]>,
    state: &mut ReleasedScanState,
    evaluated: &mut Vec<LiveProcessIdentity>,
) {
    let result = advance_scan(
        current,
        state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate.live);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    commit(state, result);
}

fn large_universe() -> Arc<[ReleasedCandidate]> {
    (usize::default()..LARGE_UNIVERSE_SIZE)
        .map(|offset| {
            candidate(
                FIRST_LARGE_PID + u32::try_from(offset).unwrap(),
                FIRST_LARGE_GENERATION + u64::try_from(offset).unwrap(),
                false,
            )
        })
        .collect::<Vec<_>>()
        .into()
}
