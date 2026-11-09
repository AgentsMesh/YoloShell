use super::*;

const CACHED_MATCH_AND_COMPANION_WORK: usize = 2;

#[test]
fn empty_scan_completes_scheduling_without_settlement_authority() {
    let result = advance_scan(
        Arc::from([]),
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
        |_candidate, _budget| unreachable!(),
    )
    .unwrap();

    assert_eq!(result.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
}

#[test]
fn stable_match_is_published_before_the_scan_completes() {
    let stable = candidate(SECOND_PID, SECOND_GENERATION, true);
    let current = candidates([candidate(FIRST_PID, FIRST_GENERATION, false), stable]);
    let mut evaluated = Vec::new();

    let result = advance_scan(
        current,
        &ReleasedScanState::default(),
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(true))
        },
    )
    .unwrap();

    assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated, [stable]);
    assert_eq!(
        result.candidates.observed,
        HashSet::from([SECOND_GENERATION])
    );
    assert_eq!(result.candidates.stable, HashSet::from([SECOND_GENERATION]));
}

#[test]
fn changed_universe_interleaves_fresh_and_prior_work() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let retired = candidate(SECOND_PID, SECOND_GENERATION, false);
    let replacement = candidate(THIRD_PID, THIRD_GENERATION, false);
    let original = candidates([first, retired]);
    let changed = candidates([first, replacement]);
    let mut state = ReleasedScanState::default();

    let initial = advance_scan(
        original,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();
    commit(&mut state, initial);
    let mut evaluated = Vec::new();
    let transition = advance_scan(
        Arc::clone(&changed),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(transition.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(transition.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated, [replacement]);
    commit(&mut state, transition);
    assert!(state.matched.is_empty());

    let next = advance_scan(
        changed,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(next.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(next.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(evaluated, [replacement, first]);
}

#[test]
fn stable_evidence_is_rematerialized_without_rescanning_a_cached_match() {
    let companion = candidate(FIRST_PID, FIRST_GENERATION, false);
    let unstable = candidate(SECOND_PID, SECOND_GENERATION, false);
    let stable = candidate(SECOND_PID, SECOND_GENERATION, true);
    let initial = candidates([companion, unstable]);
    let changed = candidates([companion, stable]);
    let mut state = ReleasedScanState::default();

    let first_batch = advance_scan(
        initial,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(true)),
    )
    .unwrap();
    commit(&mut state, first_batch);
    let mut evaluated = Vec::new();
    let rescanned = advance_scan(
        changed,
        &state,
        CACHED_MATCH_AND_COMPANION_WORK,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(rescanned.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated, [companion]);
    assert_eq!(
        rescanned.candidates.observed,
        HashSet::from([SECOND_GENERATION])
    );
    assert_eq!(
        rescanned.candidates.stable,
        HashSet::from([SECOND_GENERATION])
    );
}

#[test]
fn removed_cached_match_is_pruned_before_evidence_is_replayed() {
    let survivor = candidate(FIRST_PID, FIRST_GENERATION, false);
    let retired = candidate(SECOND_PID, SECOND_GENERATION, true);
    let replacement = candidate(THIRD_PID, THIRD_GENERATION, false);
    let initial = candidates([retired, survivor]);
    let changed = candidates([survivor, replacement]);
    let mut state = ReleasedScanState::default();

    let first_batch = advance_scan(
        initial,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(true)),
    )
    .unwrap();
    commit(&mut state, first_batch);
    let mut evaluated = Vec::new();
    let next_batch = advance_scan(
        changed,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(next_batch.candidates.status, CapabilityScanStatus::Pending);
    assert!(next_batch.candidates.observed.is_empty());
    assert_eq!(evaluated, [replacement]);
    commit(&mut state, next_batch);
    assert!(!state.matched.contains_key(&retired.live));
}
