use super::super::holder_observation::ReleasedHolderQueryStatus;
use super::*;
use crate::process::ProcessIdentity;

const HOLDER_PID: u32 = 281_001;
const AMBIENT_PID: u32 = 281_002;
const HOLDER_GENERATION: u64 = 291_001;
const REUSED_GENERATION: u64 = 291_002;
const AMBIENT_GENERATION: u64 = 291_003;
const TEST_START_TIME: u64 = 301_001;
const OBSERVED_HOLDER_COUNT: usize = 1;
const NO_OBSERVED_HOLDERS: usize = 0;
const QUERY_FAILURE_DETAIL: &str = "injected sticky holder query failure";

#[test]
fn positive_hint_survives_empty_and_failed_queries() {
    let holder = candidate(HOLDER_PID, HOLDER_GENERATION);
    let ambient = candidate(AMBIENT_PID, AMBIENT_GENERATION);
    let current = [ambient, holder];
    let first = select(
        &current,
        &ReleasedHolderHints::default(),
        ReleasedHolderQueryObservation::default(),
        holder.live,
        || Ok(HashSet::from([HOLDER_PID])),
    );
    assert_observed(&first, holder.live);

    let empty = select(
        &current,
        &first.holder_hints,
        first.observation,
        holder.live,
        || Ok(HashSet::new()),
    );
    assert_eq!(
        empty.observation,
        ReleasedHolderQueryObservation::succeeded(NO_OBSERVED_HOLDERS, NO_OBSERVED_HOLDERS)
    );
    assert_observed_priority(&empty, holder.live);

    let failed = select(
        &current,
        &empty.holder_hints,
        empty.observation,
        holder.live,
        injected_query_failure,
    );
    assert_eq!(failed.observation.status, ReleasedHolderQueryStatus::Failed);
    assert_observed_priority(&failed, holder.live);
}

#[test]
fn holder_seen_before_inventory_is_promoted_when_frontier_catches_up() {
    let holder = candidate(HOLDER_PID, HOLDER_GENERATION);
    let ambient = candidate(AMBIENT_PID, AMBIENT_GENERATION);
    let observed = select(
        &[ambient],
        &ReleasedHolderHints::default(),
        ReleasedHolderQueryObservation::default(),
        holder.live,
        || Ok(HashSet::from([HOLDER_PID])),
    );

    assert_eq!(
        observed.observation,
        ReleasedHolderQueryObservation::succeeded(OBSERVED_HOLDER_COUNT, NO_OBSERVED_HOLDERS)
    );
    assert!(observed.holder_identities.is_empty());
    assert!(observed.holder_hints.contains(&holder.live));

    let caught_up = select(
        &[ambient, holder],
        &observed.holder_hints,
        observed.observation,
        holder.live,
        || Ok(HashSet::new()),
    );
    assert_observed_priority(&caught_up, holder.live);
}

#[test]
fn reused_pid_never_inherits_hint_and_only_confirmed_exit_prunes_it() {
    let old = candidate(HOLDER_PID, HOLDER_GENERATION);
    let replacement = candidate(HOLDER_PID, REUSED_GENERATION);
    let mut hints = ReleasedHolderHints::default();
    hints.extend([old.live]);

    let matcher_failed = select_with_matcher(&[replacement], &hints, |_identity| None);
    assert!(matcher_failed.holder_hints.contains(&old.live));
    assert!(!matcher_failed.identities.contains(&replacement.live));

    let confirmed_exit =
        select_with_matcher(&[replacement], &matcher_failed.holder_hints, |identity| {
            Some(*identity != old.live)
        });
    assert!(confirmed_exit.holder_hints.identities().is_empty());
    assert!(!confirmed_exit.identities.contains(&replacement.live));
}

fn select<Q>(
    current: &[ReleasedCandidate],
    hints: &ReleasedHolderHints,
    previous: ReleasedHolderQueryObservation,
    captured: LiveProcessIdentity,
    query: Q,
) -> ReleasedPrioritySelection
where
    Q: FnOnce() -> PathHolderResult<HashSet<u32>>,
{
    released_priority_identities_with(
        current,
        MacosGlobalScanMode::TransferableWriterReleased,
        &HashSet::new(),
        hints,
        previous,
        query,
        |pid| (pid == captured.pid()).then_some(captured),
        |_identity| Some(true),
    )
}

fn select_with_matcher<M>(
    current: &[ReleasedCandidate],
    hints: &ReleasedHolderHints,
    matcher: M,
) -> ReleasedPrioritySelection
where
    M: FnMut(&LiveProcessIdentity) -> Option<bool>,
{
    released_priority_identities_with(
        current,
        MacosGlobalScanMode::TransferableWriterReleased,
        &HashSet::new(),
        hints,
        ReleasedHolderQueryObservation::default(),
        || Ok(HashSet::new()),
        |_pid| None,
        matcher,
    )
}

fn assert_observed(selection: &ReleasedPrioritySelection, holder: LiveProcessIdentity) {
    assert_eq!(
        selection.observation,
        ReleasedHolderQueryObservation::succeeded(OBSERVED_HOLDER_COUNT, OBSERVED_HOLDER_COUNT,)
    );
    assert_observed_priority(selection, holder);
}

fn assert_observed_priority(selection: &ReleasedPrioritySelection, holder: LiveProcessIdentity) {
    assert_eq!(selection.holder_identities, HashSet::from([holder]));
    assert!(selection.holder_hints.contains(&holder));
    assert!(selection.identities.contains(&holder));
}

fn injected_query_failure() -> PathHolderResult<HashSet<u32>> {
    Err(super::super::super::macos_path_holders::PathHolderError::Invariant(QUERY_FAILURE_DETAIL))
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
