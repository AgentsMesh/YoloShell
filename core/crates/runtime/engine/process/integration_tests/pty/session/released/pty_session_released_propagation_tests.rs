use super::*;

#[test]
fn replacement_round_preserves_parent_progress_across_batches() {
    let parent = candidate(THIRD_PID, THIRD_GENERATION, false);
    let pending = candidate(FIRST_PID, FIRST_GENERATION, false);
    let child = child_candidate(SECOND_PID, SECOND_GENERATION, THIRD_GENERATION, false);
    let current = candidates([parent, child]);
    let mut state = cover_first(candidates([parent, pending]), false);
    let mut evaluated = Vec::new();

    let transition = advance_scan(
        Arc::clone(&current),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(true))
        },
    )
    .unwrap();
    assert_eq!(transition.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(transition.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated, [child]);
    assert_eq!(
        transition.candidates.observed,
        HashSet::from([SECOND_GENERATION])
    );
    commit(&mut state, transition);

    let parent_batch = advance_scan(
        Arc::clone(&current),
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(true))
        },
    )
    .unwrap();

    assert_eq!(
        parent_batch.candidates.status,
        CapabilityScanStatus::Pending
    );
    assert_eq!(parent_batch.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated, [child, parent]);
    assert_eq!(
        parent_batch.candidates.observed,
        HashSet::from([SECOND_GENERATION, THIRD_GENERATION])
    );
    commit(&mut state, parent_batch);

    let retained_child = advance_scan(
        current,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(true))
        },
    )
    .unwrap();

    assert_eq!(
        retained_child.coverage_status,
        CapabilityScanStatus::Complete
    );
    assert_eq!(evaluated, [child, parent]);
    assert_eq!(
        retained_child.candidates.observed,
        HashSet::from([SECOND_GENERATION, THIRD_GENERATION])
    );
}

#[test]
fn matched_parent_does_not_authorize_a_child_across_batches() {
    let parent = candidate(THIRD_PID, THIRD_GENERATION, false);
    let pending = candidate(FIRST_PID, FIRST_GENERATION, false);
    let child = child_candidate(SECOND_PID, SECOND_GENERATION, THIRD_GENERATION, false);
    let current = candidates([parent, child]);
    let state = cover_first(candidates([parent, pending]), true);
    let mut evaluated = Vec::new();

    let transition = advance_scan(
        current,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(transition.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(transition.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated, [child]);
    assert_eq!(
        transition.candidates.observed,
        HashSet::from([THIRD_GENERATION])
    );
}

fn cover_first(current: Arc<[ReleasedCandidate]>, matches: bool) -> ReleasedScanState {
    let mut state = ReleasedScanState::default();
    let first = advance_scan(
        current,
        &state,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(matches)),
    )
    .unwrap();
    assert_eq!(first.candidates.status, CapabilityScanStatus::Pending);
    commit(&mut state, first);
    state
}
