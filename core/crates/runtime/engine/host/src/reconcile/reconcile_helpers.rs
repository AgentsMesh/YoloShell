pub(crate) fn recoverable_runtime_loss(error: &ErrorBody) -> bool {
    matches!(
        error.code.as_str(),
        crate::protocol::error_code::HOST_REBOOTED
            | crate::protocol::error_code::WORKER_UNREACHABLE
            | crate::protocol::error_code::IDENTITY_MISMATCH
    )
}

#[cfg(test)]
pub(crate) const RECONCILE_PANIC_TEST_DETAIL: &str = "injected reconciliation panic";

#[cfg(test)]
impl Host {
    pub(crate) fn panic_reconciliation_if_requested(&self, terminal_id: Uuid) {
        let selected = lock_recover(&self.catalog_test_probes.reconcile_panic).take();
        if selected == Some(terminal_id) {
            panic!("{RECONCILE_PANIC_TEST_DETAIL}");
        }
    }
}

#[cfg(test)]
pub(crate) struct ReconcileRestoreProbe {
    pub(crate) reached: tokio::sync::oneshot::Sender<AutomaticRestore>,
    pub(crate) release: tokio::sync::oneshot::Receiver<()>,
}

#[cfg(test)]
impl Host {
    pub(crate) async fn pause_after_restore_decision(&self, decision: AutomaticRestore) {
        let probe = lock_recover(&self.catalog_test_probes.restore_decision).take();
        let Some(probe) = probe else { return };
        let _ = probe.reached.send(decision);
        let _ = probe.release.await;
    }
}
