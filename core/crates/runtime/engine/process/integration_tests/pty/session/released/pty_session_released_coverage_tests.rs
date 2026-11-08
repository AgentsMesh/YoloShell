use super::*;

#[test]
fn candidate_limit_completes_coverage_without_caching_negative_proofs() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let current = candidates([first, second]);
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();
    let first_batch = advance_scan(
        Arc::clone(&current),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(
        commit(&mut state, first_batch).status,
        CapabilityScanStatus::Pending
    );

    let retried = advance_scan(
        Arc::clone(&current),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(retried.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(retried.coverage_status, CapabilityScanStatus::Complete);
    commit(&mut state, retried);
    assert_eq!(evaluated, [second, first]);
    assert!(state.matched.is_empty());

    let fresh = advance_scan(
        current,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(fresh.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated, [second, first, second]);
}

#[test]
fn candidate_addition_preserves_completed_exact_identities() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let added = candidate(THIRD_PID, THIRD_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let first_batch = advance_scan(
        candidates([first, second]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();
    commit(&mut state, first_batch);
    let mut evaluated = Vec::new();

    let next_batch = advance_scan(
        candidates([first, second, added]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(next_batch.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(next_batch.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated, [added]);
    commit(&mut state, next_batch);
    let completed = advance_scan(
        candidates([first, second, added]),
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
    assert_eq!(evaluated, [added, first]);
}

#[test]
fn deferred_round_completes_scheduling_and_retries_without_negative_evidence() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let current = candidates([first, second]);
    let mut state = ReleasedScanState::default();
    let mut evaluated = Vec::new();
    let deferred = advance_scan(
        Arc::clone(&current),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Deferred)
        },
    )
    .unwrap();
    assert_eq!(deferred.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(deferred.coverage_status, CapabilityScanStatus::Pending);
    commit(&mut state, deferred);

    let completed = advance_scan(
        current,
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(evaluated, [second, first, second, first]);
    assert_eq!(completed.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(completed.candidates.status, CapabilityScanStatus::Pending);
    assert!(completed.candidates.observed.is_empty());
    commit(&mut state, completed);

    let repeated = advance_scan(
        candidates([first, second]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(repeated.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(repeated.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated, [second, first, second, first, second, first]);
}

#[test]
fn completed_sweep_never_becomes_pty_settlement_authority() {
    let current = candidates([candidate(FIRST_PID, FIRST_GENERATION, false)]);
    let result = advance_scan(
        current,
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();

    assert_eq!(result.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
}
