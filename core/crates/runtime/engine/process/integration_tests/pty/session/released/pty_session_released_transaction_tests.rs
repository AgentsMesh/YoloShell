use super::*;
use anyhow::anyhow;

const HARD_FAILURE_DETAIL: &str = "injected released scan failure";

#[test]
fn uncommitted_advance_rescans_the_same_candidate() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([first, candidate(SECOND_PID, SECOND_GENERATION, false)]);
    let state = ReleasedScanState::default();
    let mut evaluated = Vec::new();

    for _attempt in [(), ()] {
        advance_scan(
            Arc::clone(&current),
            &state,
            ONE_CANDIDATE_PER_BATCH,
            |candidate, _budget| {
                evaluated.push(candidate);
                Ok(CandidateEvaluation::Complete(false))
            },
        )
        .unwrap();
    }

    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    assert_eq!(evaluated, [second, second]);
}

#[test]
fn deferred_candidate_is_retried_after_a_committed_match() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, true);
    let deferred = candidate(SECOND_PID, SECOND_GENERATION, false);
    let current = candidates([first, deferred]);
    let mut state = ReleasedScanState::default();

    let batch = advance_scan(
        Arc::clone(&current),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            Ok(if candidate == first {
                CandidateEvaluation::Complete(true)
            } else {
                CandidateEvaluation::Deferred
            })
        },
    )
    .unwrap();
    assert_eq!(batch.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(batch.candidates.observed, HashSet::from([FIRST_GENERATION]));
    commit(&mut state, batch);

    let mut retried = Vec::new();
    let completed = advance_scan(
        current,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            retried.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(
        completed.candidates.observed,
        HashSet::from([FIRST_GENERATION])
    );
    assert_eq!(retried, [deferred]);
}

#[test]
fn hard_error_preserves_a_previously_committed_match() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let current = candidates([first, second]);
    let mut state = ReleasedScanState::default();
    let first_batch = advance_scan(
        Arc::clone(&current),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(true)),
    )
    .unwrap();
    commit(&mut state, first_batch);

    let error = advance_scan(
        Arc::clone(&current),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |_candidate, _budget| Err(anyhow!(HARD_FAILURE_DETAIL)),
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains(HARD_FAILURE_DETAIL));

    let mut retried = Vec::new();
    advance_scan(
        current,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            retried.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(retried, [first]);
}

#[test]
fn hard_error_discards_a_match_created_in_the_failing_batch() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let current = candidates([first, second]);
    let state = ReleasedScanState::default();

    let error = advance_scan(
        Arc::clone(&current),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            if candidate == second {
                Ok(CandidateEvaluation::Complete(true))
            } else {
                Err(anyhow!(HARD_FAILURE_DETAIL))
            }
        },
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains(HARD_FAILURE_DETAIL));

    let mut retried = Vec::new();
    advance_scan(
        current,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            retried.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(retried, [second]);
}

#[test]
fn committing_complete_coverage_starts_a_fresh_pending_sweep() {
    let only = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([only]);
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();
    let completed = advance_scan(
        Arc::clone(&current),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(completed.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(completed.candidates.status, CapabilityScanStatus::Pending);
    commit(&mut state, completed);
    assert!(state.matched.is_empty());

    let repeated = advance_scan(
        current,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(repeated.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(repeated.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated, [only, only]);
}
