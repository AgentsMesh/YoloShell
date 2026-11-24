use super::*;
use crate::process::ProcessIdentity;
use std::cell::Cell;
use std::sync::Arc;

const EXACT_PID: u32 = 391_001;
const HINTED_PID: u32 = 391_002;
const AMBIENT_PID: u32 = 391_003;
const EXACT_GENERATION: u64 = 401_001;
const HINTED_GENERATION: u64 = 401_002;
const AMBIENT_GENERATION: u64 = 401_003;
const TEST_START_TIME: u64 = 411_001;
const CANDIDATE_COUNT: usize = 3;
const UNEXPECTED_FROZEN_QUERY: &str = "frozen sweep queried PTY path holders";

#[test]
fn frozen_policy_skips_query_and_completes_the_negative_snapshot() {
    let exact = candidate(EXACT_PID, EXACT_GENERATION);
    let hinted = candidate(HINTED_PID, HINTED_GENERATION);
    let ambient = candidate(AMBIENT_PID, AMBIENT_GENERATION);
    let current = [ambient, hinted, exact];
    let exact_priorities = HashSet::from([exact.live]);
    let mut hints = ReleasedHolderHints::default();
    hints.extend([hinted.live]);
    let previous = ReleasedHolderQueryObservation::succeeded(1, 1);

    let selection = released_priority_identities_with_policy(
        &current,
        MacosGlobalScanMode::TransferableWriterReleased,
        &exact_priorities,
        &hints,
        previous,
        ReleasedHolderQueryPolicy::RetainKnown,
        || panic!("{UNEXPECTED_FROZEN_QUERY}"),
        |_pid| panic!("{UNEXPECTED_FROZEN_QUERY}"),
        |_identity| Some(true),
    );

    assert_eq!(selection.observation, previous);
    assert_eq!(selection.holder_identities, HashSet::from([hinted.live]));
    assert_eq!(
        selection.identities,
        HashSet::from([exact.live, hinted.live])
    );
    assert_negative_snapshot_completes(current, selection, ambient.live);
}

#[test]
fn live_policy_refreshes_path_holder_priorities() {
    let holder = candidate(HINTED_PID, HINTED_GENERATION);
    let queried = Cell::new(false);

    let selection = released_priority_identities_with_policy(
        &[holder],
        MacosGlobalScanMode::TransferableWriterPresent,
        &HashSet::new(),
        &ReleasedHolderHints::default(),
        ReleasedHolderQueryObservation::default(),
        ReleasedHolderQueryPolicy::Refresh,
        || {
            queried.set(true);
            Ok(HashSet::from([HINTED_PID]))
        },
        |_pid| Some(holder.live),
        |_identity| Some(true),
    );

    assert!(queried.get());
    assert_eq!(selection.identities, HashSet::from([holder.live]));
    assert_eq!(
        selection.observation,
        ReleasedHolderQueryObservation::succeeded(1, 1)
    );
}

fn assert_negative_snapshot_completes(
    current: [ReleasedCandidate; CANDIDATE_COUNT],
    selection: ReleasedPrioritySelection,
    ambient: LiveProcessIdentity,
) {
    let mut state = super::super::ReleasedScanState::default();
    state.holder_hints = selection.holder_hints;
    let mut evaluated = Vec::new();
    let result = super::super::advance::advance_with_holder_lifecycle(
        Arc::from(current),
        &state,
        super::super::boundary::ReleasedPollBoundary::work_only(CANDIDATE_COUNT),
        ReleasedSweepPolicy::Frozen,
        &selection.identities,
        &selection.holder_identities,
        |candidate, _budget| {
            evaluated.push(candidate.live);
            Ok(super::super::super::macos_candidate::CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(
        result.coverage_status,
        super::super::CapabilityScanStatus::Complete
    );
    assert_eq!(evaluated.len(), CANDIDATE_COUNT);
    assert_eq!(evaluated.last(), Some(&ambient));
}

fn candidate(pid: u32, generation: u64) -> ReleasedCandidate {
    ReleasedCandidate {
        live: LiveProcessIdentity::from_kernel(pid, generation).unwrap(),
        durable: ProcessIdentity {
            pid,
            start_time: TEST_START_TIME,
        },
        parent_lineage: u64::default(),
        stable: true,
        capability_root: false,
        environment_eligible: false,
    }
}
