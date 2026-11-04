use super::*;

#[test]
fn fresh_arrival_is_promptly_interleaved_with_existing_work() {
    let added = candidate(FIRST_PID, FIRST_GENERATION, false);
    let first = candidate(SECOND_PID, SECOND_GENERATION, false);
    let pending = candidate(THIRD_PID, THIRD_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let initial = excluding(
        candidates([first, pending]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
    );
    commit(&mut state, initial);
    let mut evaluated = Vec::new();

    let continued = advance_scan(
        candidates([added, first, pending]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(evaluated, [added]);
    assert_eq!(continued.coverage_status, CapabilityScanStatus::Pending);
    commit(&mut state, continued);
    let completed = excluding(
        candidates([added, first, pending]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
    );
    assert_eq!(completed.coverage_status, CapabilityScanStatus::Complete);
}

#[test]
fn structural_change_is_promptly_interleaved_with_pending_work() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let changed = ReleasedCandidate {
        stable: true,
        ..first
    };
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let third = candidate(THIRD_PID, THIRD_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let initial = excluding(
        candidates([first, second, third]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
    );
    commit(&mut state, initial);
    let mut evaluated = Vec::new();

    let completed = advance_scan(
        candidates([changed, second, third]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(evaluated, [changed, second]);
    assert_eq!(completed.coverage_status, CapabilityScanStatus::Complete);
}

#[test]
fn fresh_arrival_finishes_before_deferred_identity_retries_next_round() {
    let added = candidate(FIRST_PID, FIRST_GENERATION, false);
    let backlog = candidate(SECOND_PID, SECOND_GENERATION, false);
    let deferred = candidate(THIRD_PID, THIRD_GENERATION, false);
    let mut state = deferred_first(deferred, backlog);
    let mut evaluated = Vec::new();

    let churned = advance_scan(
        candidates([added, backlog, deferred]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(evaluated, [added, backlog]);
    assert_eq!(churned.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(churned.coverage_status, CapabilityScanStatus::Pending);
    commit(&mut state, churned);

    let next = advance_scan(
        candidates([added, backlog, deferred]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(next.coverage_status, CapabilityScanStatus::Complete);
    assert!(evaluated.contains(&deferred));
}

#[test]
fn vanished_deferred_identity_no_longer_blocks_coverage() {
    let survivor = candidate(FIRST_PID, FIRST_GENERATION, false);
    let deferred = candidate(SECOND_PID, SECOND_GENERATION, false);
    let state = deferred_first(deferred, survivor);
    let mut evaluated = Vec::new();

    let completed = advance_scan(
        candidates([survivor]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(completed.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(evaluated, [survivor]);
}

fn deferred_first(
    deferred_candidate: ReleasedCandidate,
    pending: ReleasedCandidate,
) -> ReleasedScanState {
    let mut state = ReleasedScanState::default();
    let deferred = advance_scan(
        candidates([deferred_candidate, pending]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            Ok(if candidate == deferred_candidate {
                CandidateEvaluation::Deferred
            } else {
                CandidateEvaluation::Complete(false)
            })
        },
    )
    .unwrap();
    commit(&mut state, deferred);
    state
}

#[path = "pty_session_released_fairness_adversarial_tests.rs"]
mod fairness_adversarial_tests;
#[path = "pty_session_released_retry_fairness_tests.rs"]
mod retry_fairness_tests;
#[path = "pty_session_released_sustained_churn_tests.rs"]
mod sustained_churn_tests;

fn excluding(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    limit: usize,
) -> CapabilityScanResult {
    advance_scan(current, state, limit, |_candidate, _budget| {
        Ok(CandidateEvaluation::Complete(false))
    })
    .unwrap()
}
