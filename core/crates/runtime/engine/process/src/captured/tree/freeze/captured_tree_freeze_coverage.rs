fn refresh_freeze_coverage(tree: &mut CapturedProcessTree) -> Result<()> {
    let debt = tree.events.capture_reconciliation_debt();
    let result = tree.refresh();
    tree.events.merge_reconciliation_debt(debt);
    result.map(|_| ())
}
