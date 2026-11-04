fn consume_covered_stopped_closure(
    tree: &mut CapturedProcessTree,
) -> Result<std::ops::ControlFlow<Option<FreezeClosureCandidate>>> {
    if !tree.discovery_epoch.coverage_is_complete() {
        return Ok(std::ops::ControlFlow::Continue(()));
    }
    let targets = tree.captured_targets();
    let candidate = stopped_closure_candidate(tree, targets)?;
    if candidate.is_some() || !tree.discovery_epoch.coverage_is_complete() {
        Ok(std::ops::ControlFlow::Break(candidate))
    } else {
        Ok(std::ops::ControlFlow::Continue(()))
    }
}

fn restart_after_frozen_fork_if_required(
    tree: &mut CapturedProcessTree,
    frozen_sources: &[LiveProcessIdentity],
) -> bool {
    #[cfg(target_os = "macos")]
    if tree.events.requires_reconciliation() {
        if !tree.acknowledge_frozen_forks(frozen_sources) {
            tree.defer_frozen_discovery_for_event();
        }
        return true;
    }
    let _ = (tree, frozen_sources);
    false
}
