use super::super::super::inventory_snapshot::InventoryEntry;
use super::super::super::inventory_stable::StableInventory;
use super::super::macos_candidate::{CandidateEvaluation, fenced_candidate};
use super::super::macos_contract::{CapabilityProofError, CapabilityScanBudget};
use super::super::macos_liveness::MacosGlobalScanMode;
use super::super::macos_object_identity::MacosPtyObjectIdentity;
use super::super::macos_path_holders::PathHolderError;
use super::super::{
    CapabilityCandidates, CapabilityScanAdvance, CapabilityScanResult, CapabilityScanStatus,
    PtySessionProof,
};
use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet};

pub(super) fn capture_holder_identities(
    holders: &HashSet<u32>,
) -> Option<HashSet<crate::process::LiveProcessIdentity>> {
    capture_holder_identities_with(holders, |pid| {
        let metadata = crate::process::macos_kernel::metadata(pid).ok()??;
        crate::process::LiveProcessIdentity::from_kernel(metadata.pid, metadata.unique_id).ok()
    })
}

pub(super) fn capture_holder_identities_with<C>(
    holders: &HashSet<u32>,
    mut capture: C,
) -> Option<HashSet<crate::process::LiveProcessIdentity>>
where
    C: FnMut(u32) -> Option<crate::process::LiveProcessIdentity>,
{
    holders.iter().map(|pid| capture(*pid)).collect()
}

pub(super) fn available_candidates(
    inventory: &StableInventory,
    proof: &PtySessionProof,
    holders: &HashSet<crate::process::LiveProcessIdentity>,
    mode: MacosGlobalScanMode,
) -> Result<CapabilityScanResult> {
    let mut budget = CapabilityScanBudget::start(holders.len())?;
    let observed_identities = inventory
        .observed
        .iter()
        .map(|entry| entry.live_identity())
        .collect::<Result<HashSet<_>>>()?;
    let mut deferred = !inventory_covers_holders(&observed_identities, holders);
    let candidates = evaluate_candidates(
        &inventory.observed,
        &inventory.stable_lineages,
        holders,
        |entry| {
            let live = entry.live_identity()?;
            let outcome = fenced_candidate(
                || live.try_matches(),
                || {
                    let matches = match mode {
                        MacosGlobalScanMode::TransferableWriterReleased => {
                            proof.matches_available_after_writer_release(entry.pid, &mut budget)
                        }
                        _ => proof.matches_available_process(entry.pid, &mut budget),
                    };
                    matches
                        .map_err(|error| error.with_candidate_identity(entry.pid, live.unique_id()))
                },
            )?;
            match outcome {
                CandidateEvaluation::Complete(matches) => Ok(matches),
                CandidateEvaluation::Deferred | CandidateEvaluation::Retry => {
                    deferred = true;
                    Ok(false)
                }
            }
        },
    )?;
    Ok(finish_available_scan(candidates, deferred))
}

pub(super) fn inventory_covers_holders(
    observed: &HashSet<crate::process::LiveProcessIdentity>,
    holders: &HashSet<crate::process::LiveProcessIdentity>,
) -> bool {
    holders.is_subset(observed)
}

pub(super) fn finish_available_scan(
    mut candidates: CapabilityCandidates,
    deferred: bool,
) -> CapabilityScanResult {
    let status = if deferred {
        CapabilityScanStatus::Pending
    } else {
        CapabilityScanStatus::Complete
    };
    candidates.status = status;
    CapabilityScanResult {
        candidates,
        advance: CapabilityScanAdvance::reset(),
        scheduling_status: status,
        coverage_status: status,
    }
}

pub(super) fn require_stable_available_fence(result: &mut CapabilityScanResult, stable: bool) {
    if !stable {
        result.candidates.status = CapabilityScanStatus::Pending;
        result.scheduling_status = CapabilityScanStatus::Pending;
        result.coverage_status = CapabilityScanStatus::Pending;
    }
}

pub(super) fn available_fence_stable(
    expected_path: MacosPtyObjectIdentity,
    before_path: MacosPtyObjectIdentity,
    before_pids: &HashSet<u32>,
    before: &HashSet<crate::process::LiveProcessIdentity>,
    after_path: Option<MacosPtyObjectIdentity>,
    after_pids: Option<&HashSet<u32>>,
    after: Option<&HashSet<crate::process::LiveProcessIdentity>>,
) -> bool {
    before_path == expected_path
        && after_path == Some(expected_path)
        && after_pids.is_some_and(|holders| holders == before_pids)
        && after.is_some_and(|holders| holders == before)
}

pub(super) fn evaluate_candidates<F>(
    observed: &[InventoryEntry],
    stable_lineages: &HashSet<u64>,
    holders: &HashSet<crate::process::LiveProcessIdentity>,
    mut proof_matches: F,
) -> Result<CapabilityCandidates>
where
    F: FnMut(InventoryEntry) -> Result<bool>,
{
    let mut filtered = HashMap::new();
    for entry in observed.iter().copied() {
        if holders.contains(&entry.live_identity()?) {
            filtered.insert(entry.lineage_id(), entry);
        }
    }
    let mut candidates = CapabilityCandidates::default();
    for (lineage, entry) in filtered {
        if !proof_matches(entry)? {
            continue;
        }
        candidates.observed.insert(lineage);
        if stable_lineages.contains(&lineage) {
            candidates.stable.insert(lineage);
        }
    }
    Ok(candidates)
}

pub(super) fn map_path_holder_error(error: PathHolderError) -> CapabilityProofError {
    match error {
        PathHolderError::Query(error) => CapabilityProofError::holder_query(error),
        PathHolderError::BudgetExceeded(detail) => CapabilityProofError::budget(anyhow!(detail)),
        PathHolderError::Invariant(detail) => CapabilityProofError::invariant(anyhow!(detail)),
    }
}
