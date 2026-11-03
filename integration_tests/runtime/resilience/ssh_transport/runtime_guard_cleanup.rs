use super::*;

const DETACHED_CLEANUP_AUTHORITY_MISSING_DETAIL: &str =
    "detached runtime cleanup authority is missing";

impl DetachedRuntimeGuard {
    pub(in super::super) fn bind_authenticated_inventory(
        &mut self,
        views: &[yolo_runtime::TerminalView],
    ) -> Result<()> {
        let boot_id = self
            .boot_id
            .as_deref()
            .context(DETACHED_CLEANUP_AUTHORITY_MISSING_DETAIL)?;
        let cleanup = self
            .cleanup
            .as_mut()
            .context(DETACHED_CLEANUP_AUTHORITY_MISSING_DETAIL)?;
        let active = views
            .iter()
            .filter(|view| authorizes_cleanup(view))
            .collect::<Vec<_>>();
        cleanup.bind_views_for_boot(&self.paths, &active, boot_id)?;
        self.rescan_cleanup()
    }

    pub(in super::super) fn track_running_runtime(
        &mut self,
        view: &yolo_runtime::TerminalView,
    ) -> Result<()> {
        let boot_id = self
            .boot_id
            .as_deref()
            .context(DETACHED_CLEANUP_AUTHORITY_MISSING_DETAIL)?;
        let cleanup = self
            .cleanup
            .as_mut()
            .context(DETACHED_CLEANUP_AUTHORITY_MISSING_DETAIL)?;
        cleanup.bind_view_for_boot(&self.paths, view, boot_id)
    }

    #[cfg(test)]
    pub(in super::super) fn cleanup_authority_counts_for_test(&self) -> (usize, usize, usize) {
        self.cleanup.as_ref().map_or((0, 0, 0), |cleanup| {
            (
                cleanup.bound_records_for_test().len(),
                cleanup.pending_records_for_test().len(),
                cleanup.tree_identities_for_test().len(),
            )
        })
    }

    #[cfg(test)]
    pub(in super::super) fn cleanup_tree_contains_view_for_test(
        &self,
        view: &yolo_runtime::TerminalView,
    ) -> bool {
        let Some(runtime) = view.runtime.as_ref() else {
            return false;
        };
        let identity = yolo_process_test_support::ProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        };
        self.cleanup
            .as_ref()
            .is_some_and(|cleanup| cleanup.tree_identities_for_test().contains(&identity))
    }

    pub(in super::super) async fn shutdown(&mut self) -> Result<()> {
        let mut failures = Vec::new();
        remember_failure(&mut failures, self.rescan_cleanup());
        if let Some(cleanup) = self.cleanup.as_mut() {
            cleanup.request();
        }
        remember_failure(&mut failures, stop_host_async(self.host).await);
        if let Some(cleanup) = self.cleanup.as_mut() {
            cleanup.force();
            remember_failure(&mut failures, cleanup.finish());
        }
        self.armed = cleanup_requires_retry(self.host.observe(), !failures.is_empty());
        finish_failures(failures)
    }

    fn rescan_cleanup(&mut self) -> Result<()> {
        let boot_id = self
            .boot_id
            .as_deref()
            .context(DETACHED_CLEANUP_AUTHORITY_MISSING_DETAIL)?;
        let cleanup = self
            .cleanup
            .as_mut()
            .context(DETACHED_CLEANUP_AUTHORITY_MISSING_DETAIL)?;
        cleanup.rescan_for_boot(&self.paths, boot_id)
    }
}

fn authorizes_cleanup(view: &yolo_runtime::TerminalView) -> bool {
    view.runtime.as_ref().is_some_and(|runtime| {
        matches!(
            runtime.status,
            yolo_runtime::TerminalRuntimeStatus::Starting
                | yolo_runtime::TerminalRuntimeStatus::Running
                | yolo_runtime::TerminalRuntimeStatus::Finalizing
                | yolo_runtime::TerminalRuntimeStatus::OutputBlocked
        )
    })
}

impl Drop for DetachedRuntimeGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let mut failures = Vec::new();
        remember_failure(&mut failures, self.rescan_cleanup());
        if let Some(cleanup) = self.cleanup.as_mut() {
            cleanup.request();
        }
        remember_failure(&mut failures, signal_if_matching(self.host, libc::SIGKILL));
        remember_failure(&mut failures, wait_for_host_gone(self.host));
        if let Some(cleanup) = self.cleanup.as_mut() {
            cleanup.force();
            remember_failure(&mut failures, cleanup.finish());
        }
        self.armed = cleanup_requires_retry(self.host.observe(), !failures.is_empty());
        report_drop_failures(failures);
    }
}
