use super::*;

impl RuntimeCleanup {
    pub(crate) fn capture_for_boot(paths: &Paths, expected_boot_id: &str) -> (Self, Result<()>) {
        let candidates::RuntimeRecordDiscovery {
            records,
            unbound,
            mut issues,
        } = candidates::discover_runtime_records_for_boot(paths, expected_boot_id, &[]);
        let mut trees = RuntimeProcessTrees::default();
        issues.record(trees.ensure_cleanup_records(paths, &records));
        let capture = issues.check();
        (
            Self {
                records,
                unbound,
                trees,
                issues,
            },
            capture,
        )
    }

    pub(crate) fn rescan_for_boot(&mut self, paths: &Paths, expected_boot_id: &str) -> Result<()> {
        let candidates::RuntimeRecordDiscovery {
            mut records,
            mut unbound,
            issues,
        } = candidates::discover_runtime_records_for_boot(paths, expected_boot_id, &self.records);
        let mut operation = issues;
        candidates::retain_unsettled_pending(paths, &mut unbound, &self.unbound);
        operation.record(deduplicate_records(&mut records));
        operation.record(candidates::reject_unbound_records(&unbound));
        let update = operation.record(self.trees.prepare_cleanup_records(paths, &records));
        let result = operation.check();
        if result.is_ok()
            && let Some(update) = update
        {
            self.records = records;
            self.unbound.clear();
            self.trees.commit(update);
        }
        self.issues.merge(operation);
        result
    }
}
