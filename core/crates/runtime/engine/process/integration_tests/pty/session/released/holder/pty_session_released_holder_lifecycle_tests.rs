use super::*;

#[test]
fn writer_present_completion_and_restart_preserve_hint_until_released_conclusion() {
    let target = candidate(THIRD_PID, THIRD_GENERATION, false);
    let ambient = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([ambient, target]);
    let mut state = state_with_holder(target.live);
    let mut evaluated = Vec::new();

    let present_priority = empty_holder_priority(
        &current,
        &state,
        MacosGlobalScanMode::TransferableWriterPresent,
    );
    let present = advance_holder_selection(
        Arc::clone(&current),
        &state,
        MacosGlobalScanMode::TransferableWriterPresent,
        present_priority,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    );
    commit(&mut state, present);
    assert_eq!(state.holder_hints_for_test(), HashSet::from([target.live]));

    state = state.restart_preserving_matches();
    assert_eq!(state.round_size_for_test(), usize::default());
    assert_eq!(
        state.holder_observation,
        holder_observation::ReleasedHolderQueryObservation::default()
    );
    assert_eq!(state.holder_hints_for_test(), HashSet::from([target.live]));

    let released_priority = empty_holder_priority(
        &current,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
    );
    let released = advance_holder_selection(
        current,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
        released_priority,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    );
    commit(&mut state, released);

    assert_eq!(evaluated, [target, target]);
    assert!(state.holder_hints_for_test().is_empty());
    assert!(!state.covers_exact_closure(&[target.live]));
}

#[test]
fn retry_and_deferred_retain_released_holder_hint() {
    let target = candidate(THIRD_PID, THIRD_GENERATION, false);
    for evaluation in [CandidateEvaluation::Retry, CandidateEvaluation::Deferred] {
        let mut state = state_with_holder(target.live);
        let current = candidates([target]);
        let priority = empty_holder_priority(
            &current,
            &state,
            MacosGlobalScanMode::TransferableWriterReleased,
        );
        let result = advance_holder_selection(
            current,
            &state,
            MacosGlobalScanMode::TransferableWriterReleased,
            priority,
            ONE_CANDIDATE_PER_BATCH,
            |_candidate, _budget| Ok(evaluation),
        );
        commit(&mut state, result);

        assert_eq!(state.holder_hints_for_test(), HashSet::from([target.live]));
    }
}

#[test]
fn deferred_holder_retries_before_ambient_after_empty_query() {
    let target = candidate(THIRD_PID, THIRD_GENERATION, false);
    let ambient = candidate(FIRST_PID, FIRST_GENERATION, false);
    let current = candidates([ambient, target]);
    let mut state = state_with_holder(target.live);
    let mut evaluated = Vec::new();

    let first_priority = empty_holder_priority(
        &current,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
    );
    let first = advance_holder_selection(
        Arc::clone(&current),
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
        first_priority,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Deferred)
        },
    );
    commit(&mut state, first);

    let second_priority = empty_holder_priority(
        &current,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
    );
    let second = advance_holder_selection(
        current,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
        second_priority,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    );
    commit(&mut state, second);

    assert_eq!(evaluated, [target, target]);
    assert!(state.holder_hints_for_test().is_empty());
    assert_eq!(
        state.pending_lineages_for_test(),
        [ambient.live.lineage_id()]
    );
}

#[test]
fn released_positive_match_requires_evaluation_and_consumes_hint() {
    let target = candidate(THIRD_PID, THIRD_GENERATION, true);
    let current = candidates([target]);
    let mut state = state_with_holder(target.live);
    assert!(!state.covers_exact_closure(&[target.live]));
    let priority = empty_holder_priority(
        &current,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
    );

    let result = advance_holder_selection(
        current,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
        priority,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(true)),
    );
    commit(&mut state, result);

    assert!(state.holder_hints_for_test().is_empty());
    assert!(state.covers_exact_closure(&[target.live]));
}

fn state_with_holder(identity: LiveProcessIdentity) -> ReleasedScanState {
    let mut state = ReleasedScanState::default();
    state.holder_hints.extend([identity]);
    state
}
