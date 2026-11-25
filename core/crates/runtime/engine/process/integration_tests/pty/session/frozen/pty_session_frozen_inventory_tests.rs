use super::super::boundary::RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL;
use super::*;
use crate::process::inventory_observation::cache;
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_stable::STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE;
use crate::process::pty_session_proof::PtyCapabilityScanState;

const LARGE_FROZEN_UNIVERSE_SIZE: usize = 830;
const LARGE_FROZEN_FIRST_PID: u32 = 510_000;
const LARGE_FROZEN_FIRST_GENERATION: u64 = 520_000;

#[test]
fn active_frozen_round_retains_the_same_inventory() {
    let current = candidates([
        candidate(FIRST_PID, FIRST_GENERATION, false),
        candidate(SECOND_PID, SECOND_GENERATION, false),
    ]);
    let mut state = PtyCapabilityScanState::default();
    let _inventory = inventory_fixture(1);
    let captured = cache::capture(&state).unwrap();
    let mut result = advance_with_policy(
        current,
        &state.released,
        ONE_CANDIDATE_PER_BATCH,
        ReleasedSweepPolicy::Frozen,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();

    cache::retain(&mut result, &captured);
    state.commit(result.advance);
    let reused = cache::capture(&state).unwrap();

    assert!(Arc::ptr_eq(&captured, &reused));
}

#[test]
fn live_round_does_not_retain_inventory() {
    let current = candidates([
        candidate(FIRST_PID, FIRST_GENERATION, false),
        candidate(SECOND_PID, SECOND_GENERATION, false),
    ]);
    let state = PtyCapabilityScanState::default();
    let _inventory = inventory_fixture(1);
    let captured = cache::capture(&state).unwrap();
    let mut result = advance_with_policy(
        current,
        &state.released,
        ONE_CANDIDATE_PER_BATCH,
        ReleasedSweepPolicy::Live,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();

    cache::retain(&mut result, &captured);

    assert!(result.advance.next.frozen_inventory().is_none());
}

#[test]
fn completed_frozen_round_releases_inventory_immediately() {
    let current = candidates([candidate(FIRST_PID, FIRST_GENERATION, false)]);
    let state = PtyCapabilityScanState::default();
    let _inventory = inventory_fixture(1);
    let captured = cache::capture(&state).unwrap();
    let mut result = advance_with_policy(
        current,
        &state.released,
        ONE_CANDIDATE_PER_BATCH,
        ReleasedSweepPolicy::Frozen,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();

    cache::retain(&mut result, &captured);

    assert_eq!(result.coverage_status, CapabilityScanStatus::Complete);
    assert!(result.advance.next.frozen_inventory().is_none());
}

#[test]
fn large_frozen_round_captures_one_inventory_across_all_polls() {
    let current = large_frozen_universe();
    let mut state = PtyCapabilityScanState::default();
    let _inventory = inventory_fixture(1);
    let mut first_inventory = None;
    let mut evaluated = usize::default();
    let mut polls = usize::default();

    loop {
        polls += 1;
        let captured = cache::capture(&state).unwrap();
        let expected = first_inventory.get_or_insert_with(|| Arc::clone(&captured));
        assert!(Arc::ptr_eq(expected, &captured));
        let mut result = advance_with_policy(
            Arc::clone(&current),
            &state.released,
            RELEASED_FROZEN_SWEEP_MINIMUM_WORK_PER_POLL,
            ReleasedSweepPolicy::Frozen,
            |_candidate, _budget| {
                evaluated += 1;
                Ok(CandidateEvaluation::Complete(false))
            },
        )
        .unwrap();
        let complete = result.coverage_status == CapabilityScanStatus::Complete;
        cache::retain(&mut result, &captured);
        state.commit(result.advance);
        if complete {
            break;
        }
    }

    assert!(polls > 1);
    assert_eq!(evaluated, LARGE_FROZEN_UNIVERSE_SIZE);
    assert!(!state.has_frozen_inventory_for_test());
}

fn inventory_fixture(captures: usize) -> crate::process::inventory_snapshot::TestInventoryGuard {
    install_test_inventory(std::iter::repeat_n(
        Vec::new(),
        STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * captures,
    ))
}

fn large_frozen_universe() -> Arc<[ReleasedCandidate]> {
    (usize::default()..LARGE_FROZEN_UNIVERSE_SIZE)
        .map(|offset| {
            candidate(
                LARGE_FROZEN_FIRST_PID + u32::try_from(offset).unwrap(),
                LARGE_FROZEN_FIRST_GENERATION + u64::try_from(offset).unwrap(),
                false,
            )
        })
        .collect::<Vec<_>>()
        .into()
}
