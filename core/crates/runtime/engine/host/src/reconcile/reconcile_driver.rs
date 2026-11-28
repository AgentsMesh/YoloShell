pub(crate) const RECONCILIATION_FAILURE_COUNT: usize = 1;
pub(crate) const RECONCILIATION_PANIC_MESSAGE: &str =
    "host reconciliation panicked and was isolated";
pub(crate) const SESSION_RECONCILIATION_PANIC_MESSAGE: &str =
    "session reconciliation panicked and was isolated";

impl Host {
    pub async fn reconcile(&self) -> ReconcileResult {
        match terminal_containment::catch_future(self.reconcile_inner(true)).await {
            Ok(result) => result,
            Err(()) => {
                terminal_containment::trace(|| {
                    tracing::error!(RECONCILIATION_PANIC_MESSAGE);
                });
                isolated_reconciliation_failure()
            }
        }
    }

    pub(crate) async fn reconcile_inner(&self, discover: bool) -> ReconcileResult {
        let _reconcile_guard = self.reconcile_gate.lock().await;
        let batch = (!discover && !self.recovery_queue.needs_discovery())
            .then(|| self.recovery_queue.pending());
        if batch.as_ref().is_some_and(Vec::is_empty)
            && lock_recover(&self.pending_recovery_writes).is_empty()
            && lock_recover(&self.pending_attachment_cleanup).is_empty()
        {
            return ReconcileResult::default();
        }
        if let Err(error) = self.verify_namespace() {
            tracing::error!(%error, "host reconciliation namespace was lost");
            return isolated_reconciliation_failure();
        }
        let ids = batch
            .as_ref()
            .map(|tickets| tickets.iter().map(|ticket| ticket.terminal_id).collect());
        let (manifests, diagnostics) = match self.recovery_manifests(ids).await {
            Ok(listing) => listing,
            Err(error) => {
                tracing::error!(%error, "cannot read sessions for recovery");
                return isolated_reconciliation_failure();
            }
        };
        let mut result = ReconcileResult {
            unreachable: diagnostics,
            ..Default::default()
        };
        let tickets = self
            .recovery_queue
            .selected(batch, manifests.into_iter().map(|m| m.terminal_id));
        let sweeps = tickets.into_iter().map(|ticket| async move {
            let terminal_id = ticket.terminal_id;
            if let Err(error) = self.verify_namespace() {
                tracing::error!(%error, %terminal_id, "host reconciliation namespace changed");
                return isolated_reconciliation_failure();
            }
            contain_session_reconciliation(terminal_id, self.reconcile_ticket(ticket)).await
        });
        use futures::StreamExt as _;
        let mut sweeps = futures::stream::iter(sweeps).buffer_unordered(MAX_RECONCILE_CONCURRENCY);
        while let Some(session_result) = sweeps.next().await {
            result.merge(session_result);
        }
        result.unreachable += self.retry_pending_recovery_writes().await;
        result.unreachable += self.retry_archived_attachment_cleanup().await;
        result
    }
}

pub(crate) fn contain_session_reconciliation(
    terminal_id: Uuid,
    future: impl std::future::Future<Output = ReconcileResult>,
) -> impl std::future::Future<Output = ReconcileResult> {
    let caught = terminal_containment::catch_future(future);
    async move {
        match caught.await {
            Ok(result) => result,
            Err(()) => {
                terminal_containment::trace(|| {
                    tracing::error!(%terminal_id, SESSION_RECONCILIATION_PANIC_MESSAGE);
                });
                isolated_reconciliation_failure()
            }
        }
    }
}

pub(crate) fn isolated_reconciliation_failure() -> ReconcileResult {
    ReconcileResult {
        unreachable: RECONCILIATION_FAILURE_COUNT,
        ..ReconcileResult::default()
    }
}
