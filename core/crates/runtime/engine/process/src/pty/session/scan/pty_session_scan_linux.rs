use super::super::constants::LINUX_PTY_PROOF_CANDIDATES_PER_BATCH;
use super::super::inventory_entry::InventoryEntry;
use super::super::inventory_stable::StableInventory;
use super::linux_contract::{
    BatchBudget, BudgetKind, CandidateEvaluation, CandidatePending, CandidateProof, QueryKind,
};
use super::linux_scan_state::{
    LinuxScanState, MatchedCandidate, batch_end, materialize, retained_authority_limit,
};
use super::{
    CapabilityCandidates, CapabilityScanAdvance, CapabilityScanResult, CapabilityScanStatus,
    PtySessionProof,
};
use crate::process::{ProcessIdentity, identity::process_identity_is_inactive};
use std::collections::HashSet;
use std::sync::Arc;
#[path = "pty_session_scan_linux_inventory.rs"]
mod inventory_scan;
pub(super) use inventory_scan::scan;

enum EvaluatedCandidate {
    Excluded,
    Matched(MatchedCandidate),
    Pending(CandidatePending),
}

fn advance_with<F>(
    current: &[InventoryEntry],
    mut next: LinuxScanState,
    unstable: bool,
    authority_limit: usize,
    mut evaluate: F,
) -> (CapabilityCandidates, LinuxScanState, bool)
where
    F: FnMut(InventoryEntry, &mut BatchBudget) -> EvaluatedCandidate,
{
    let start = next.cursor.min(current.len());
    let end = batch_end(start, current.len(), LINUX_PTY_PROOF_CANDIDATES_PER_BATCH);
    let mut blocked = next.blocked || unstable;
    if unstable {
        next.pending = Some(CandidatePending::IdentityChanged);
    }
    let mut suppressed = HashSet::new();
    let mut budget = BatchBudget::new();
    for entry in current[start..end].iter().copied() {
        let identity = entry.identity();
        if next.excluded.contains(&identity) {
            continue;
        }
        if let Some(matched) = next.matched.get(&identity).cloned() {
            match matched.live.try_matches() {
                Ok(true) => continue,
                Ok(false) => {
                    next.matched.remove(&identity);
                    suppressed.insert(identity);
                    next.pending = Some(CandidatePending::Exited);
                }
                Err(_) => {
                    suppressed.insert(identity);
                    next.pending = Some(CandidatePending::Query(QueryKind::Identity));
                }
            }
            blocked = true;
            continue;
        }
        match evaluate(entry, &mut budget) {
            EvaluatedCandidate::Excluded => {
                next.excluded.insert(identity);
            }
            EvaluatedCandidate::Matched(matched) if next.matched.len() < authority_limit => {
                next.matched.insert(identity, matched);
            }
            EvaluatedCandidate::Matched(_matched) => {
                next.pending = Some(CandidatePending::Budget(BudgetKind::RetainedAuthorities));
                blocked = true;
            }
            EvaluatedCandidate::Pending(reason) => {
                next.pending = Some(reason);
                blocked = true;
            }
        }
    }
    next.cursor = end;
    next.blocked = blocked;
    let mut candidates = materialize(&next, &suppressed);
    if end < current.len() {
        candidates.status = CapabilityScanStatus::Pending;
    } else if blocked {
        candidates.status = CapabilityScanStatus::Pending;
        next.cursor = usize::default();
        next.blocked = false;
    } else {
        candidates.status = CapabilityScanStatus::Complete;
        next = LinuxScanState::default();
    }
    (candidates, next, start < end)
}

fn evaluate(
    entry: InventoryEntry,
    proof: &PtySessionProof,
    owner: u32,
    budget: &mut BatchBudget,
) -> EvaluatedCandidate {
    let live = match entry.live_identity() {
        Ok(Some(live)) => live,
        Ok(None) => return inactive_evaluation(entry),
        Err(_) => {
            return EvaluatedCandidate::Pending(CandidatePending::Query(QueryKind::Identity));
        }
    };
    match proof.evaluate_linux_candidate(&live, owner, budget) {
        CandidateEvaluation::Complete(CandidateProof::Excluded) => EvaluatedCandidate::Excluded,
        CandidateEvaluation::Complete(CandidateProof::Matched) => {
            EvaluatedCandidate::Matched(MatchedCandidate {
                live,
                durable: entry.identity(),
            })
        }
        CandidateEvaluation::Pending(reason) => EvaluatedCandidate::Pending(reason),
    }
}

fn inactive_evaluation(entry: InventoryEntry) -> EvaluatedCandidate {
    match process_identity_is_inactive(entry.pid, entry.start_time) {
        Ok(true) => EvaluatedCandidate::Excluded,
        Ok(false) => EvaluatedCandidate::Pending(CandidatePending::IdentityChanged),
        Err(error) if process_gone(&error) => EvaluatedCandidate::Excluded,
        Err(_) => EvaluatedCandidate::Pending(CandidatePending::Query(QueryKind::Identity)),
    }
}

fn process_gone(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .and_then(std::io::Error::raw_os_error)
        .is_some_and(|code| code == libc::ENOENT || code == libc::ESRCH)
}

fn result(
    candidates: CapabilityCandidates,
    next: LinuxScanState,
    batch_consumed: bool,
) -> CapabilityScanResult {
    let coverage_status = candidates.status;
    CapabilityScanResult {
        candidates,
        advance: CapabilityScanAdvance::linux(next).with_consumed_batch(batch_consumed),
        scheduling_status: coverage_status,
        coverage_status,
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/scan/pty_session_scan_linux_authority_tests.rs"]
mod authority_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/scan/pty_session_scan_linux_tests.rs"]
mod tests;
