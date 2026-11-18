use super::super::runtime_record_authority::BoundRuntimeRecord;
use super::*;
use anyhow::{Context, Result};

const READ_BOUND_RUNTIME_CONTEXT: &str = "read trusted runtime cleanup record";

impl RuntimeCleanup {
    pub(crate) fn bind_views_for_boot(
        &mut self,
        paths: &Paths,
        views: &[&yolo_runtime::TerminalView],
        expected_boot_id: &str,
    ) -> Result<()> {
        let mut operation = CleanupIssues::default();
        let mut candidates = self.records.clone();
        for view in views {
            let terminal_id = view.manifest.terminal_id;
            let bound = read_json(&paths.runtime_record(terminal_id))
                .context(READ_BOUND_RUNTIME_CONTEXT)
                .and_then(|record| {
                    BoundRuntimeRecord::bind_view_for_boot(paths, record, view, expected_boot_id)
                });
            if let Some(bound) = operation.record(bound) {
                candidates.push(bound);
            }
        }
        let validation = operation.check();
        if validation.is_err() {
            self.issues.merge(operation);
            return validation;
        }
        self.commit_bound_candidates(paths, candidates, operation)
    }

    pub(crate) fn bind_view(
        &mut self,
        paths: &Paths,
        view: &yolo_runtime::TerminalView,
    ) -> Result<()> {
        let terminal_id = view.manifest.terminal_id;
        let bound = read_json(&paths.runtime_record(terminal_id))
            .context(READ_BOUND_RUNTIME_CONTEXT)
            .and_then(|record| BoundRuntimeRecord::bind_view(paths, record, view));
        self.bind_result(paths, bound)
    }

    pub(crate) fn bind_view_for_boot(
        &mut self,
        paths: &Paths,
        view: &yolo_runtime::TerminalView,
        expected_boot_id: &str,
    ) -> Result<()> {
        let terminal_id = view.manifest.terminal_id;
        let bound = read_json(&paths.runtime_record(terminal_id))
            .context(READ_BOUND_RUNTIME_CONTEXT)
            .and_then(|record| {
                BoundRuntimeRecord::bind_view_for_boot(paths, record, view, expected_boot_id)
            });
        self.bind_result(paths, bound)
    }

    fn bind_result(&mut self, paths: &Paths, result: Result<BoundRuntimeRecord>) -> Result<()> {
        let Some(bound) = self.issues.record(result) else {
            return self.issues.check();
        };
        self.bind_record(paths, bound)
    }

    fn bind_record(&mut self, paths: &Paths, bound: BoundRuntimeRecord) -> Result<()> {
        let operation = CleanupIssues::default();
        let mut candidates = self.records.clone();
        candidates.push(bound);
        self.commit_bound_candidates(paths, candidates, operation)
    }

    fn commit_bound_candidates(
        &mut self,
        paths: &Paths,
        mut candidates: Vec<BoundRuntimeRecord>,
        mut operation: CleanupIssues,
    ) -> Result<()> {
        operation.record(deduplicate_records(&mut candidates));
        let update = operation.record(self.trees.prepare_cleanup_records(paths, &candidates));
        let result = operation.check();
        if result.is_ok()
            && let Some(update) = update
        {
            self.records = candidates;
            self.remove_bound_pending();
            self.trees.commit(update);
        }
        self.issues.merge(operation);
        result
    }

    fn remove_bound_pending(&mut self) {
        self.unbound.retain(|candidate| {
            !self
                .records
                .iter()
                .any(|bound| bound.record().has_same_process_identity(candidate))
        });
    }
}
