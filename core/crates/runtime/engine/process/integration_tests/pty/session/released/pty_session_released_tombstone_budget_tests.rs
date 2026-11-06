use super::super::boundary::RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL;
use super::*;

const TOMBSTONE_UNIVERSE_SIZE: usize = 4_096;
const ZERO_WORK_PER_POLL: usize = 0;
const FIRST_TOMBSTONE_PID: u32 = 81_000;
const FIRST_TOMBSTONE_GENERATION: u64 = 91_000;
const REPLACEMENT_PID: u32 = 86_000;
const REPLACEMENT_GENERATION: u64 = 96_000;

#[test]
fn tombstone_cleanup_reserves_work_for_the_live_replacement() {
    let original = tombstone_universe();
    let mut state = ReleasedScanState::default();
    let initial = advance_scan(
        original,
        &state,
        ZERO_WORK_PER_POLL,
        |_candidate, _budget| unreachable!(),
    )
    .unwrap();
    assert_eq!(initial.coverage_status, CapabilityScanStatus::Pending);
    commit(&mut state, initial);
    assert_eq!(state.queued_entries_for_test(), TOMBSTONE_UNIVERSE_SIZE);

    let replacement = candidate(REPLACEMENT_PID, REPLACEMENT_GENERATION, false);
    let current = candidates([replacement]);
    let mut evaluated = Vec::new();
    let bounded = advance_scan(
        Arc::clone(&current),
        &state,
        RELEASED_LIVE_SWEEP_MINIMUM_WORK_PER_POLL,
        |candidate, _budget| {
            evaluated.push(candidate.live);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();
    assert_eq!(evaluated, [replacement.live]);
    assert_eq!(bounded.coverage_status, CapabilityScanStatus::Complete);
    commit(&mut state, bounded);
    assert_eq!(state.queued_entries_for_test(), usize::default());
    assert!(state.pending_lineages_for_test().is_empty());
}

fn tombstone_universe() -> Arc<[ReleasedCandidate]> {
    (usize::default()..TOMBSTONE_UNIVERSE_SIZE)
        .map(|offset| {
            candidate(
                FIRST_TOMBSTONE_PID + u32::try_from(offset).unwrap(),
                FIRST_TOMBSTONE_GENERATION + u64::try_from(offset).unwrap(),
                false,
            )
        })
        .collect::<Vec<_>>()
        .into()
}
