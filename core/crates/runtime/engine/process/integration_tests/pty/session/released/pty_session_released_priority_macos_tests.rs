use super::super::super::macos_candidate::CandidateEvaluation;
use super::super::holder_observation::ReleasedHolderQueryStatus;
use super::*;
use crate::process::ProcessIdentity;
use std::sync::Arc;

const HOLDER_PID: u32 = 73_001;
const OTHER_PID: u32 = 73_002;
const HOLDER_GENERATION: u64 = 83_001;
const OTHER_GENERATION: u64 = 83_002;
const START_TIME: u64 = 93_001;
const ONE_CANDIDATE_PER_POLL: usize = 1;
const TEST_CAPABILITY: &str = "released-priority-test-capability";
const INJECTED_QUERY_FAILURE: &str = "injected released path-holder query failure";
const UNEXPECTED_HOLDER_QUERY: &str = "path-holder query ran outside transferable modes";
const RETURNED_HOLDER_PID_COUNT: usize = 2;

#[test]
fn priority_hints_cover_both_transferable_writer_states() {
    assert!(uses_transferable_universe(
        MacosGlobalScanMode::TransferableWriterPresent
    ));
    assert!(uses_transferable_universe(
        MacosGlobalScanMode::TransferableWriterReleased
    ));
    assert!(!uses_transferable_universe(
        MacosGlobalScanMode::LineageEnvironment
    ));
}

#[test]
fn failed_query_and_successful_empty_query_have_distinct_observations() {
    let current = [candidate(HOLDER_PID, HOLDER_GENERATION)];
    let failed = selection_with(&current, || injected_query_failure());
    let empty = selection_with(&current, || Ok(HashSet::new()));

    assert_eq!(failed.observation, ReleasedHolderQueryObservation::failed());
    assert_eq!(
        empty.observation,
        ReleasedHolderQueryObservation::succeeded(usize::default(), usize::default())
    );
}

#[test]
fn successful_query_reports_holder_and_inventory_match_counts() {
    let holder = candidate(HOLDER_PID, HOLDER_GENERATION);
    let other = candidate(OTHER_PID, OTHER_GENERATION);
    let selection = selection_with(&[holder, other], || {
        Ok(HashSet::from([HOLDER_PID, u32::MAX]))
    });

    assert_eq!(
        selection.observation,
        ReleasedHolderQueryObservation::succeeded(
            RETURNED_HOLDER_PID_COUNT,
            ONE_CANDIDATE_PER_POLL,
        )
    );
    assert_eq!(selection.identities, HashSet::from([holder.live]));
}

#[test]
fn holder_query_failure_keeps_the_full_scan_schedule() {
    let holder = candidate(HOLDER_PID, HOLDER_GENERATION);
    let other = candidate(OTHER_PID, OTHER_GENERATION);
    let current = [holder, other];
    let selection = selection_with(&current, || injected_query_failure());
    let scheduled = super::super::schedule::ordered_schedule(&current, &selection.identities);

    assert_eq!(scheduled.len(), current.len());
    assert_eq!(
        scheduled
            .into_iter()
            .map(|item| item.live)
            .collect::<HashSet<_>>(),
        HashSet::from([holder.live, other.live])
    );
}

#[test]
fn holder_diagnostic_does_not_create_match_authority() {
    let holder = candidate(HOLDER_PID, HOLDER_GENERATION);
    let selection = selection_with(&[holder], || Ok(HashSet::from([HOLDER_PID])));
    let result = super::super::advance::advance_with_priority_boundary(
        Arc::from([holder]),
        &super::super::ReleasedScanState::default(),
        super::super::boundary::ReleasedPollBoundary::work_only(ONE_CANDIDATE_PER_POLL),
        super::super::coverage::ReleasedSweepPolicy::Frozen,
        &selection.identities,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(false)),
    )
    .unwrap();

    assert!(result.candidates.identities.is_empty());
    assert!(
        result
            .advance
            .next
            .released_matched_lineages_for_test()
            .is_empty()
    );
}

#[test]
fn lineage_environment_preserves_caller_exact_priority() {
    let holder = candidate(HOLDER_PID, HOLDER_GENERATION);
    let other = candidate(OTHER_PID, OTHER_GENERATION);
    let proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();

    let result = super::super::advance_candidate_round(
        Arc::from([holder, other]),
        &super::super::ReleasedScanState::default(),
        &proof,
        MacosGlobalScanMode::LineageEnvironment,
        super::super::candidate::GlobalCandidateContext::Escaped,
        super::super::coverage::ReleasedSweepPolicy::Live,
        &HashSet::from([holder.live]),
        &HashSet::new(),
        super::super::boundary::ReleasedPollBoundary::work_only(ONE_CANDIDATE_PER_POLL),
    )
    .unwrap();

    assert_eq!(
        result.advance.next.released_pending_lineages_for_test(),
        [other.live.lineage_id()]
    );
}

#[test]
fn lineage_environment_does_not_query_or_add_path_holder_hint() {
    let holder = candidate(HOLDER_PID, HOLDER_GENERATION);
    let other = candidate(OTHER_PID, OTHER_GENERATION);
    let exact = HashSet::from([holder.live]);

    let selection = released_priority_identities_with(
        &[other],
        MacosGlobalScanMode::LineageEnvironment,
        &exact,
        &ReleasedHolderHints::default(),
        ReleasedHolderQueryObservation::default(),
        || panic!("{UNEXPECTED_HOLDER_QUERY}"),
        |_pid| None,
        |_identity| Some(true),
    );

    assert_eq!(selection.identities, exact);
    assert_eq!(
        selection.observation.status,
        ReleasedHolderQueryStatus::NotAttempted
    );
}

fn selection_with<Q>(current: &[ReleasedCandidate], query: Q) -> ReleasedPrioritySelection
where
    Q: FnOnce() -> PathHolderResult<HashSet<u32>>,
{
    released_priority_identities_with(
        current,
        MacosGlobalScanMode::TransferableWriterReleased,
        &HashSet::new(),
        &ReleasedHolderHints::default(),
        ReleasedHolderQueryObservation::default(),
        query,
        |pid| {
            current
                .iter()
                .find(|candidate| candidate.live.pid() == pid)
                .map(|candidate| candidate.live)
        },
        |_identity| Some(true),
    )
}

fn injected_query_failure<T>() -> PathHolderResult<T> {
    Err(super::super::super::macos_path_holders::PathHolderError::Invariant(INJECTED_QUERY_FAILURE))
}

fn candidate(pid: u32, generation: u64) -> ReleasedCandidate {
    ReleasedCandidate {
        live: LiveProcessIdentity::from_kernel(pid, generation).unwrap(),
        durable: ProcessIdentity {
            pid,
            start_time: START_TIME,
        },
        parent_lineage: u64::default(),
        stable: true,
        capability_root: false,
        environment_eligible: false,
    }
}
