use super::*;

const OBSERVED_HOLDER_COUNT: usize = 1;
const FRONTIER_MATCH_COUNT: usize = 0;
const INITIAL_FROZEN_ROUND_SIZE: usize = 2;

#[test]
fn holder_seen_during_inventory_omission_joins_the_same_frozen_round() {
    let first = candidate(FIRST_PID, FIRST_GENERATION, false);
    let second = candidate(SECOND_PID, SECOND_GENERATION, false);
    let holder = candidate(THIRD_PID, THIRD_GENERATION, false);
    let initial = candidates([first, second]);
    let mut state = ReleasedScanState::default();
    let observed = select_holder_priority_with(
        &initial,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
        || Ok(HashSet::from([holder.live.pid()])),
        |pid| (pid == holder.live.pid()).then_some(holder.live),
        |_identity| Some(true),
    );
    assert_eq!(
        observed.observation,
        holder_observation::ReleasedHolderQueryObservation::succeeded(
            OBSERVED_HOLDER_COUNT,
            FRONTIER_MATCH_COUNT,
        )
    );
    assert!(observed.holder_identities.is_empty());

    let first_poll = advance_holder_selection(
        initial,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
        observed,
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    );
    commit(&mut state, first_poll);
    assert_eq!(state.round_size_for_test(), INITIAL_FROZEN_ROUND_SIZE);
    assert_eq!(state.holder_hints_for_test(), HashSet::from([holder.live]));

    let caught_up = candidates([first, second, holder]);
    let empty = empty_holder_priority(
        &caught_up,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
    );
    let mut evaluated = Vec::new();
    let second_poll = advance_holder_selection(
        caught_up,
        &state,
        MacosGlobalScanMode::TransferableWriterReleased,
        empty,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, _budget| {
            evaluated.push(candidate);
            Ok(CandidateEvaluation::Complete(false))
        },
    );
    commit(&mut state, second_poll);

    assert_eq!(evaluated, [holder]);
    assert!(state.holder_hints_for_test().is_empty());
    assert!(!state.covers_exact_closure(&[holder.live]));
}
