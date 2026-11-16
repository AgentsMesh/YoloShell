use super::*;
use crate::process::LiveProcessIdentity;

type Context<'a> = (
    ProcessIdentity,
    bool,
    &'a HashSet<LiveProcessIdentity>,
    &'a HashSet<LiveProcessIdentity>,
);

pub(in super::super) fn scan(
    inventory: &StableInventory,
    proof: &PtySessionProof,
    state: &LinuxScanState,
    context: Context<'_>,
) -> anyhow::Result<CapabilityScanResult> {
    let owner = unsafe { libc::geteuid() };
    scan_with(inventory, proof, state, context, |entry, budget| {
        evaluate(entry, proof, owner, budget)
    })
}

fn scan_with(
    inventory: &StableInventory,
    proof: &PtySessionProof,
    state: &LinuxScanState,
    context: Context<'_>,
    evaluate: impl FnMut(InventoryEntry, &mut BatchBudget) -> EvaluatedCandidate,
) -> anyhow::Result<CapabilityScanResult> {
    let (root, root_exited, priorities, excluded) = context;
    let owner = unsafe { libc::geteuid() };
    let excluded: HashSet<_> = excluded
        .iter()
        .map(LiveProcessIdentity::durable_projection)
        .collect();
    let retained: HashSet<_> = priorities
        .iter()
        .map(LiveProcessIdentity::durable_projection)
        .chain(state.matched.keys().copied())
        .collect();
    let relevant = |entry: &&InventoryEntry| {
        entry.effective_user_id == owner
            && !(root_exited && entry.identity() == root)
            && !excluded.contains(&entry.identity())
            && proof.linux_candidate_relevant(**entry, root, retained.contains(&entry.identity()))
    };
    let mut current = inventory
        .entries
        .iter()
        .filter(relevant)
        .copied()
        .collect::<Vec<_>>();
    current.sort_unstable_by_key(|entry| (entry.pid, entry.start_time));
    let universe = current
        .iter()
        .map(|entry| entry.identity())
        .collect::<Vec<_>>();
    let stable = universe.iter().copied().collect::<HashSet<_>>();
    let unstable = inventory
        .observed
        .iter()
        .filter(relevant)
        .any(|entry| !stable.contains(&entry.identity()));
    let next = state.reconcile(Arc::from(universe));
    let Some(authority_limit) = retained_authority_limit(next.matched.len()) else {
        let mut next = next;
        next.pending = Some(CandidatePending::Budget(BudgetKind::RetainedAuthorities));
        let mut candidates = materialize(&next, &HashSet::new());
        candidates.status = CapabilityScanStatus::Pending;
        return Ok(result(candidates, next, false));
    };
    let (candidates, next, consumed) =
        advance_with(&current, next, unstable, authority_limit, evaluate);
    Ok(result(candidates, next, consumed))
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../../unit_tests/pty_session_scan_linux_origin_tests.rs"]
mod tests;
