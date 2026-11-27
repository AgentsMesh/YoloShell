use super::ReleasedScanState;
use super::priority::ReleasedPrioritySelection;
use super::universe::ReleasedCandidate;
use crate::process::LiveProcessIdentity;
use crate::process::pty_session_proof::macos_liveness::MacosGlobalScanMode;
use std::collections::HashSet;
use std::sync::Arc;

pub(super) fn exact(
    mode: MacosGlobalScanMode,
    priority: &ReleasedPrioritySelection,
) -> Option<HashSet<LiveProcessIdentity>> {
    (matches!(
        mode,
        MacosGlobalScanMode::CustodyPtyObject | MacosGlobalScanMode::RecoveryCustodyPtyObject
    ) && priority.observation.covers_inventory()
        && (priority.observation.path_is_missing()
            || !priority.observed_holder_identities.is_empty()))
    .then(|| priority.observed_holder_identities.clone())
}

pub(super) fn constrain_candidates(
    current: Arc<[ReleasedCandidate]>,
    exact: Option<&HashSet<LiveProcessIdentity>>,
) -> Arc<[ReleasedCandidate]> {
    let Some(exact) = exact else {
        return current;
    };
    current
        .iter()
        .filter(|candidate| exact.contains(&candidate.live))
        .copied()
        .collect::<Vec<_>>()
        .into()
}

pub(super) fn validates(
    mode: MacosGlobalScanMode,
    expected: Option<&HashSet<LiveProcessIdentity>>,
    state: &ReleasedScanState,
) -> bool {
    match expected {
        Some(expected) => state.matches_all(expected),
        // Recovery is backed by a typed, durably attested custody fence. When
        // the path-holder hint is unavailable, the full stable inventory still
        // receives exact generation and PTY-object descriptor verification.
        None => mode != MacosGlobalScanMode::CustodyPtyObject,
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/released/pty_session_released_custody_snapshot_macos_tests.rs"]
mod tests;
