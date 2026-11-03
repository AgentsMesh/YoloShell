#[derive(Clone, Copy)]
enum FreshInspectionMode {
    Ordinary,
    KernelEof,
}

pub(super) fn inspect_fresh(
    tree: &mut CapturedProcessTree,
) -> Result<Option<FreezeClosureCandidate>> {
    #[cfg(target_os = "linux")]
    if tree.linux_containment_available() {
        return tree
            .linux_frozen_closure(false)
            .map(|closure| closure.map(FreezeClosureCandidate::Ordinary));
    }
    fresh_inspection(tree, FreshInspectionMode::Ordinary)
}

pub(super) fn inspect_with_kernel_eof_fresh(
    tree: &mut CapturedProcessTree,
) -> Result<Option<FreezeClosureCandidate>> {
    fresh_inspection(tree, FreshInspectionMode::KernelEof)
}

fn fresh_inspection(
    tree: &mut CapturedProcessTree,
    mode: FreshInspectionMode,
) -> Result<Option<FreezeClosureCandidate>> {
    let debt = tree.events.capture_reconciliation_debt();
    let result = match mode {
        FreshInspectionMode::Ordinary => tree.refresh(),
        FreshInspectionMode::KernelEof => tree.refresh_for_kernel_eof(),
    };
    tree.events.merge_reconciliation_debt(debt);
    result?;
    if let Some(candidate) = exact_stopped_candidate(tree)? {
        return Ok(Some(candidate));
    }
    let targets = tree.captured_targets();
    stopped_closure_candidate(tree, targets)
}
