#[cfg(test)]
use super::super::constants::MAX_PTY_PROOF_CANDIDATES;
use super::super::inventory_stable::StableInventory;
#[cfg(test)]
use super::CapabilityCandidates;
#[cfg(test)]
use super::macos_candidate::CandidateEvaluation;
#[cfg(test)]
use super::macos_contract::CapabilityScanBudget;
use super::macos_liveness::MacosGlobalScanMode;
use super::{CapabilityScanResult, CapabilityScanStatus, PtySessionProof};
use crate::process::LiveProcessIdentity;
use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
#[path = "pty_session_released_advance_macos.rs"]
mod advance;
#[path = "pty_session_released_boundary_macos.rs"]
mod boundary;
#[path = "pty_session_released_candidate_macos.rs"]
mod candidate;
#[path = "pty_session_released_coverage_macos.rs"]
mod coverage;
#[path = "pty_session_released_custody_snapshot_macos.rs"]
mod custody_snapshot;
#[path = "pty_session_released_holder_hints_macos.rs"]
mod holder_hints;
#[path = "pty_session_released_holder_observation_macos.rs"]
mod holder_observation;
#[path = "pty_session_released_priority_macos.rs"]
mod priority;
#[path = "pty_session_released_queue_macos.rs"]
mod queue;
#[path = "pty_session_released_result_macos.rs"]
mod result;
#[path = "pty_session_released_schedule_macos.rs"]
mod schedule;
#[path = "pty_session_released_state_macos.rs"]
mod state;
#[path = "pty_session_released_universe_macos.rs"]
mod universe;
#[cfg(all(test, feature = "yolo-integration-tests"))]
use advance::advance_scan;
#[cfg(all(test, feature = "yolo-integration-tests"))]
use advance::advance_with_boundary;
#[cfg(all(test, feature = "yolo-integration-tests"))]
use advance::advance_with_policy;
use boundary::ReleasedPollBoundary;
use candidate::{GlobalCandidateContext, evaluate_global_candidate};
use coverage::ReleasedSweepPolicy;
pub(super) use state::ReleasedScanState;
use universe::{ReleasedCandidate, scan_candidate_universe};

pub(super) fn scan_released(
    inventory: &StableInventory,
    proof: &PtySessionProof,
    state: &ReleasedScanState,
    minimum_start_time: u64,
    root_lineage: u64,
    mode: MacosGlobalScanMode,
    excluded: &HashSet<LiveProcessIdentity>,
) -> Result<CapabilityScanResult> {
    let current = scan_candidate_universe(inventory, mode, minimum_start_time, root_lineage)?;
    let current = universe::exclude_candidates(current, excluded);
    let state = state.excluding(excluded);
    let policy = sweep_policy(false, mode);
    advance_candidate_round(
        current,
        &state,
        proof,
        mode,
        GlobalCandidateContext::ReleasedTty,
        policy,
        &HashSet::new(),
        &HashSet::new(),
        production_boundary(policy),
    )
}

pub(super) fn scan_escaped(
    inventory: &StableInventory,
    proof: &PtySessionProof,
    state: &ReleasedScanState,
    minimum_start_time: u64,
    root_lineage: u64,
    mode: MacosGlobalScanMode,
    freeze_round: bool,
    exact_priorities: &HashSet<LiveProcessIdentity>,
    excluded: &HashSet<LiveProcessIdentity>,
) -> Result<CapabilityScanResult> {
    let current = scan_candidate_universe(inventory, mode, minimum_start_time, root_lineage)?;
    let current = universe::exclude_candidates(current, excluded);
    let state = state.excluding(excluded);
    let policy = sweep_policy(freeze_round, mode);
    let priority = priority::released_priority_identities(
        proof,
        &current,
        mode,
        exact_priorities,
        &state.holder_hints,
        state.holder_observation,
        priority::ReleasedHolderQueryPolicy::for_sweep(policy, mode),
    );
    let custody_snapshot = custody_snapshot::exact(mode, &priority);
    let current = custody_snapshot::constrain_candidates(current, custody_snapshot.as_ref());
    let mut priority_state = state;
    priority_state.holder_hints = priority.holder_hints;
    let mut result = advance_candidate_round(
        current,
        &priority_state,
        proof,
        mode,
        GlobalCandidateContext::Escaped,
        policy,
        &priority.identities,
        &priority.holder_identities,
        production_boundary(policy),
    )?;
    result.advance.next.released.holder_observation = priority.observation;
    let custody_snapshot_valid = custody_snapshot::validates(
        mode,
        custody_snapshot.as_ref(),
        &result.advance.next.released,
    );
    if !custody_snapshot_valid {
        result.coverage_status = CapabilityScanStatus::Pending;
    }
    if has_global_exclusion_authority(mode)
        && result.coverage_status == CapabilityScanStatus::Complete
        && custody_snapshot_valid
    {
        result.candidates.status = CapabilityScanStatus::Complete;
    }
    Ok(result)
}

fn has_global_exclusion_authority(mode: MacosGlobalScanMode) -> bool {
    // Pipe EOF closes the discovery membership universe. Exact identities matched
    // while a writer existed remain authoritative until that generation exits.
    matches!(mode, MacosGlobalScanMode::ReleasedDiscoveryRetained)
}

fn advance_candidate_round(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    proof: &PtySessionProof,
    mode: MacosGlobalScanMode,
    context: GlobalCandidateContext,
    policy: ReleasedSweepPolicy,
    priority_identities: &HashSet<LiveProcessIdentity>,
    holder_identities: &HashSet<LiveProcessIdentity>,
    boundary: ReleasedPollBoundary,
) -> Result<CapabilityScanResult> {
    let empty_holder_identities = HashSet::new();
    let conclusive_holder_identities = if matches!(
        mode,
        MacosGlobalScanMode::TransferableWriterReleased
            | MacosGlobalScanMode::RecoveryCustodyPtyObject
    ) {
        holder_identities
    } else {
        &empty_holder_identities
    };
    advance::advance_with_holder_lifecycle(
        current,
        state,
        boundary,
        policy,
        priority_identities,
        conclusive_holder_identities,
        |candidate, budget| evaluate_global_candidate(proof, candidate, budget, context, mode),
    )
}

fn production_boundary(policy: ReleasedSweepPolicy) -> ReleasedPollBoundary {
    match policy {
        ReleasedSweepPolicy::Live => ReleasedPollBoundary::live_production(),
        ReleasedSweepPolicy::Frozen => ReleasedPollBoundary::frozen_production(),
    }
}

fn sweep_policy(freeze_round: bool, mode: MacosGlobalScanMode) -> ReleasedSweepPolicy {
    if freeze_round || mode == MacosGlobalScanMode::PrematureBootstrapRelease {
        ReleasedSweepPolicy::Frozen
    } else {
        ReleasedSweepPolicy::Live
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/released/scan/pty_session_released_scan_tests.rs"]
mod tests;
