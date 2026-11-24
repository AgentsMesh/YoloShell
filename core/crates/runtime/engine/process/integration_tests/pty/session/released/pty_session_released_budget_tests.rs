use super::*;
use crate::process::constants::{
    MAX_PTY_PROOF_FILE_DESCRIPTORS, MAX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS,
};
use crate::process::pty_session_proof::macos_candidate::fenced_candidate;

const OVERFLOW_CANDIDATE_INCREMENT: usize = 1;
const FULL_DESCRIPTOR_BATCH_CANDIDATES: usize =
    MAX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS / MAX_PTY_PROOF_FILE_DESCRIPTORS;
const BUDGET_TEST_CANDIDATES: usize =
    FULL_DESCRIPTOR_BATCH_CANDIDATES + OVERFLOW_CANDIDATE_INCREMENT;
const RETRY_SEQUENCE_CANDIDATES: usize = 2;

#[test]
fn frozen_production_aggregate_exhaustion_resumes_at_the_overflow_candidate() {
    let current = budget_candidates();
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();

    let first_batch = advance_with_boundary(
        Arc::clone(&current),
        &state,
        boundary::ReleasedPollBoundary::frozen_production(),
        ReleasedSweepPolicy::Frozen,
        |candidate, budget| {
            evaluated.push(candidate);
            descriptor_heavy_exclusion(budget)
        },
    )
    .unwrap();

    assert_eq!(first_batch.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(first_batch.coverage_status, CapabilityScanStatus::Pending);
    assert!(first_batch.advance.batch_consumed);
    commit(&mut state, first_batch);
    assert!(state.matched.is_empty());
    assert_eq!(evaluated.len(), BUDGET_TEST_CANDIDATES);
    evaluated.clear();

    let retried = advance_with_boundary(
        Arc::clone(&current),
        &state,
        boundary::ReleasedPollBoundary::frozen_production(),
        ReleasedSweepPolicy::Frozen,
        |candidate, budget| {
            evaluated.push(candidate);
            descriptor_heavy_exclusion(budget)
        },
    )
    .unwrap();

    assert_eq!(retried.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(retried.coverage_status, CapabilityScanStatus::Complete);
    assert!(retried.advance.batch_consumed);
    assert_eq!(evaluated.len(), OVERFLOW_CANDIDATE_INCREMENT);
    commit(&mut state, retried);
    assert!(state.matched.is_empty());
}

#[test]
fn empty_round_does_not_claim_a_consumed_batch() {
    let result = advance_scan(
        Arc::from([]),
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
        |_candidate, _budget| unreachable!(),
    )
    .unwrap();

    assert!(!result.advance.batch_consumed);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Complete);
}

#[test]
fn retry_on_first_candidate_reports_no_consumed_progress() {
    let result = advance_scan(
        candidates([candidate(FIRST_PID, FIRST_GENERATION, false)]),
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
        |_candidate, _budget| Ok(CandidateEvaluation::Retry),
    )
    .unwrap();

    assert!(!result.advance.batch_consumed);
}

#[test]
fn retry_reports_only_progress_consumed_before_that_candidate() {
    let current = candidates([
        candidate(FIRST_PID, FIRST_GENERATION, false),
        candidate(SECOND_PID, SECOND_GENERATION, false),
    ]);
    let mut evaluations = usize::default();
    let result = advance_scan(
        current,
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
        |_candidate, _budget| {
            evaluations += 1;
            if evaluations == RETRY_SEQUENCE_CANDIDATES {
                Ok(CandidateEvaluation::Retry)
            } else {
                Ok(CandidateEvaluation::Complete(false))
            }
        },
    )
    .unwrap();

    assert_eq!(evaluations, RETRY_SEQUENCE_CANDIDATES);
    assert!(result.advance.batch_consumed);
}

#[test]
fn retained_match_removed_from_schedule_counts_as_progress() {
    let retained = candidate(FIRST_PID, FIRST_GENERATION, false);
    let state = ReleasedScanState {
        matched: [(retained.live, retained)].into_iter().collect(),
        ..ReleasedScanState::default()
    };
    let result = advance_scan(
        candidates([retained]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |_candidate, _budget| panic!("retained match was reevaluated"),
    )
    .unwrap();

    assert!(result.advance.batch_consumed);
}

#[test]
fn removed_remaining_candidate_counts_as_consumed_coverage() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let first_batch = advance_scan(
        candidates([first, second]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();
    commit(&mut state, first_batch);

    let reconciled = advance_scan(
        candidates([second]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| panic!("completed candidate was reevaluated"),
    )
    .unwrap();

    assert_eq!(reconciled.coverage_status, CapabilityScanStatus::Complete);
    assert!(reconciled.advance.batch_consumed);
}

fn descriptor_heavy_exclusion(budget: &mut CapabilityScanBudget) -> Result<CandidateEvaluation> {
    fenced_candidate(
        || Ok(true),
        || {
            budget.admit_descriptors(MAX_PTY_PROOF_FILE_DESCRIPTORS)?;
            Ok(false)
        },
    )
}

fn budget_candidates() -> Arc<[ReleasedCandidate]> {
    (usize::default()..BUDGET_TEST_CANDIDATES)
        .map(|offset| {
            candidate(
                FIRST_PID + u32::try_from(offset).unwrap(),
                FIRST_GENERATION + u64::try_from(offset).unwrap(),
                false,
            )
        })
        .collect::<Vec<_>>()
        .into()
}
