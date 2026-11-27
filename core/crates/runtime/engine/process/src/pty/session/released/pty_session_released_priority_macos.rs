use super::super::super::constants::MAX_PTY_PROOF_CANDIDATES;
use super::super::PtySessionProof;
use super::super::macos_liveness::MacosGlobalScanMode;
use super::super::macos_path_holders::{PathHolderResult, path_holders};
use super::coverage::ReleasedSweepPolicy;
use super::holder_hints::ReleasedHolderHints;
use super::holder_observation::ReleasedHolderQueryObservation;
use super::universe::ReleasedCandidate;
use crate::process::LiveProcessIdentity;
use std::collections::HashSet;

#[path = "pty_session_released_priority_types_macos.rs"]
mod types;
pub(super) use types::{ReleasedHolderQueryPolicy, ReleasedPrioritySelection, selection};

pub(super) fn released_priority_identities(
    proof: &PtySessionProof,
    current: &[ReleasedCandidate],
    mode: MacosGlobalScanMode,
    exact: &HashSet<LiveProcessIdentity>,
    previous_hints: &ReleasedHolderHints,
    previous: ReleasedHolderQueryObservation,
    query_policy: ReleasedHolderQueryPolicy,
) -> ReleasedPrioritySelection {
    released_priority_identities_with_policy(
        current,
        mode,
        exact,
        previous_hints,
        previous,
        query_policy,
        || path_holders(&proof.tty_path, MAX_PTY_PROOF_CANDIDATES),
        capture_holder_identity,
        |identity| identity.try_matches().ok(),
    )
}

#[cfg(test)]
pub(super) fn released_priority_identities_with<Q, C, M>(
    current: &[ReleasedCandidate],
    mode: MacosGlobalScanMode,
    exact: &HashSet<LiveProcessIdentity>,
    previous_hints: &ReleasedHolderHints,
    previous: ReleasedHolderQueryObservation,
    query_holders: Q,
    capture_identity: C,
    generation_matches: M,
) -> ReleasedPrioritySelection
where
    Q: FnOnce() -> PathHolderResult<HashSet<u32>>,
    C: FnMut(u32) -> Option<LiveProcessIdentity>,
    M: FnMut(&LiveProcessIdentity) -> Option<bool>,
{
    released_priority_identities_with_policy(
        current,
        mode,
        exact,
        previous_hints,
        previous,
        ReleasedHolderQueryPolicy::Refresh,
        query_holders,
        capture_identity,
        generation_matches,
    )
}

pub(super) fn released_priority_identities_with_policy<Q, C, M>(
    current: &[ReleasedCandidate],
    mode: MacosGlobalScanMode,
    exact: &HashSet<LiveProcessIdentity>,
    previous_hints: &ReleasedHolderHints,
    previous: ReleasedHolderQueryObservation,
    query_policy: ReleasedHolderQueryPolicy,
    query_holders: Q,
    mut capture_identity: C,
    generation_matches: M,
) -> ReleasedPrioritySelection
where
    Q: FnOnce() -> PathHolderResult<HashSet<u32>>,
    C: FnMut(u32) -> Option<LiveProcessIdentity>,
    M: FnMut(&LiveProcessIdentity) -> Option<bool>,
{
    let mut priorities = exact.clone();
    let mut holder_hints = previous_hints.reconcile_with(generation_matches);
    if !uses_transferable_universe(mode) {
        return selection(
            priorities,
            HashSet::new(),
            HashSet::new(),
            holder_hints,
            ReleasedHolderQueryObservation::default(),
        );
    }
    if query_policy == ReleasedHolderQueryPolicy::RetainKnown {
        let holder_identities = holder_hints.priorities_for(current);
        priorities.extend(holder_identities.iter().copied());
        return selection(
            priorities,
            holder_identities,
            HashSet::new(),
            holder_hints,
            previous,
        );
    }
    // This query is only a positive scheduling hint. Every returned generation
    // still passes the normal identity and descriptor fences before it can match.
    let holder_pids = match query_holders() {
        Ok(holders) => holders,
        Err(error) if error.is_missing_path() => {
            return selection(
                priorities,
                HashSet::new(),
                HashSet::new(),
                ReleasedHolderHints::default(),
                ReleasedHolderQueryObservation::missing_path(),
            );
        }
        Err(error) => {
            let observation = ReleasedHolderQueryObservation::failed();
            observation.warn_failure_transition(previous, current.len(), &error);
            let holder_identities = holder_hints.priorities_for(current);
            priorities.extend(holder_identities.iter().copied());
            return selection(
                priorities,
                holder_identities,
                HashSet::new(),
                holder_hints,
                observation,
            );
        }
    };
    let observed_holders = holder_pids
        .iter()
        .filter_map(|pid| capture_identity(*pid))
        .collect::<HashSet<_>>();
    let matched_count = current
        .iter()
        .filter(|candidate| observed_holders.contains(&candidate.live))
        .count();
    let observed_holder_identities = current
        .iter()
        .map(|candidate| candidate.live)
        .filter(|identity| observed_holders.contains(identity))
        .collect::<HashSet<_>>();
    holder_hints.extend(observed_holders);
    let holder_identities = holder_hints.priorities_for(current);
    let observation = ReleasedHolderQueryObservation::succeeded(holder_pids.len(), matched_count);
    observation.trace(current.len());
    priorities.extend(holder_identities.iter().copied());
    selection(
        priorities,
        holder_identities,
        observed_holder_identities,
        holder_hints,
        observation,
    )
}

pub(super) fn uses_transferable_universe(mode: MacosGlobalScanMode) -> bool {
    matches!(
        mode,
        MacosGlobalScanMode::CustodyPtyObject
            | MacosGlobalScanMode::RecoveryCustodyPtyObject
            | MacosGlobalScanMode::TransferableWriterPresent
            | MacosGlobalScanMode::TransferableWriterReleased
    )
}

fn capture_holder_identity(pid: u32) -> Option<LiveProcessIdentity> {
    crate::process::macos_native::try_kernel_process_metadata(pid)
        .ok()
        .flatten()
        .and_then(|metadata| {
            LiveProcessIdentity::from_kernel(metadata.pid, metadata.unique_id).ok()
        })
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/released/holder/pty_session_released_holder_priority_macos_tests.rs"]
mod holder_priority_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/released/pty_session_released_query_policy_macos_tests.rs"]
mod query_policy_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/released/pty_session_released_priority_macos_tests.rs"]
mod tests;
