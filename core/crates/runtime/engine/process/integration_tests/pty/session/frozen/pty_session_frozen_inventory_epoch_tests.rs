use super::*;
use crate::process::inventory_observation::cache;
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_stable::STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE;
use crate::process::pty_session_proof::PtyCapabilityScanState;

const FROZEN_EPOCH_CAPTURE_COUNT: usize = 2;

#[test]
fn restart_preserves_matches_and_starts_with_a_fresh_inventory() {
    let current = candidates([
        candidate(FIRST_PID, FIRST_GENERATION, false),
        candidate(SECOND_PID, SECOND_GENERATION, false),
    ]);
    let mut state = PtyCapabilityScanState::default();
    let _inventory = inventory_fixture();
    let first = cache::capture(&state).unwrap();
    let mut first_result = advance_with_policy(
        Arc::clone(&current),
        &state.released,
        ONE_CANDIDATE_PER_BATCH,
        ReleasedSweepPolicy::Frozen,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(true)),
    )
    .unwrap();
    cache::retain(&mut first_result, &first);
    state.commit(first_result.advance);
    let matched = state.released_matched_lineages_for_test();

    state.restart();
    let second = cache::capture(&state).unwrap();
    let mut second_result = advance_with_policy(
        current,
        &state.released,
        ONE_CANDIDATE_PER_BATCH,
        ReleasedSweepPolicy::Frozen,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();
    cache::retain(&mut second_result, &second);

    assert_eq!(state.released_matched_lineages_for_test(), matched);
    assert!(!matched.is_empty());
    assert!(!Arc::ptr_eq(&first, &second));
    let retained = second_result.advance.next.frozen_inventory().unwrap();
    assert!(Arc::ptr_eq(&second, &retained));
}

#[test]
fn deferred_round_does_not_reuse_its_inventory() {
    let current = candidates([candidate(FIRST_PID, FIRST_GENERATION, false)]);
    let state = PtyCapabilityScanState::default();
    let _inventory = inventory_fixture();
    let first = cache::capture(&state).unwrap();
    let mut result = advance_with_policy(
        current,
        &state.released,
        ONE_CANDIDATE_PER_BATCH,
        ReleasedSweepPolicy::Frozen,
        |_candidate, _budget| Ok(CandidateEvaluation::Deferred),
    )
    .unwrap();
    cache::retain(&mut result, &first);
    let second = cache::capture(&result.advance.next).unwrap();

    assert_eq!(result.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
    assert!(!Arc::ptr_eq(&first, &second));
}

fn inventory_fixture() -> crate::process::inventory_snapshot::TestInventoryGuard {
    install_test_inventory(std::iter::repeat_n(
        Vec::new(),
        STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * FROZEN_EPOCH_CAPTURE_COUNT,
    ))
}
