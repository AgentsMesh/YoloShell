use super::*;

#[test]
fn negative_proof_becoming_a_match_is_re_evaluated_in_the_next_sweep() {
    let companion = candidate(FIRST_PID, FIRST_GENERATION, false);
    let target = candidate(SECOND_PID, SECOND_GENERATION, true);
    let current = candidates([companion, target]);
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
    assert_eq!(first_batch.candidates.status, CapabilityScanStatus::Pending);
    commit(&mut state, first_batch);

    let covered = advance_scan(
        Arc::clone(&current),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(candidate == target))
        },
    )
    .unwrap();
    assert_eq!(covered.coverage_status, CapabilityScanStatus::Complete);
    assert!(covered.candidates.observed.is_empty());
    commit(&mut state, covered);

    let changed = advance_scan(
        current,
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(candidate == target))
        },
    )
    .unwrap();

    assert_eq!(changed.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(
        changed.candidates.observed,
        HashSet::from([SECOND_GENERATION])
    );
    assert_eq!(
        changed.candidates.stable,
        HashSet::from([SECOND_GENERATION])
    );
    assert_eq!(evaluated, [target, companion, target, companion]);
}

#[test]
fn candidate_opened_between_empty_sweeps_is_published() {
    let candidate = candidate(FIRST_PID, FIRST_GENERATION, true);
    let current = candidates([candidate]);
    let mut state = ReleasedScanState::default();
    let empty = advance_scan(
        Arc::clone(&current),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();
    commit(&mut state, empty);

    let nonempty = advance_scan(
        current,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(true)),
    )
    .unwrap();
    assert_eq!(nonempty.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(
        nonempty.candidates.observed,
        HashSet::from([FIRST_GENERATION])
    );
}

#[test]
fn exact_pid_generation_replacement_is_scanned_in_the_next_sweep() {
    let original = candidate(FIRST_PID, FIRST_GENERATION, false);
    let replacement = candidate(FIRST_PID, SECOND_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let original_scan = advance_scan(
        candidates([original]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();
    commit(&mut state, original_scan);

    let mut evaluated = Vec::new();
    let replacement_scan = excluding_batch(candidates([replacement]), &state, &mut evaluated);
    assert_eq!(
        replacement_scan.candidates.status,
        CapabilityScanStatus::Pending
    );
    assert_eq!(evaluated, [replacement]);
}

#[test]
fn replaced_snapshot_member_preserves_unchanged_progress() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let removed = candidate(SECOND_PID, SECOND_GENERATION, false);
    let successor = candidate(SECOND_PID, THIRD_GENERATION, false);
    let tail = candidate(THIRD_PID, FOURTH_GENERATION, false);
    let mut state = ReleasedScanState::default();
    let initial = advance_scan(
        candidates([tail, removed, first]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();
    commit(&mut state, initial);

    let mut evaluated = Vec::new();
    let changed = advance_scan(
        candidates([tail, first, successor]),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(changed.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(changed.coverage_status, CapabilityScanStatus::Pending);
    commit(&mut state, changed);

    assert_eq!(evaluated, [successor]);
}

fn excluding_batch(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    evaluated: &mut Vec<ReleasedCandidate>,
) -> CapabilityScanResult {
    advance_scan(
        current,
        state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap()
}
