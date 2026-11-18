use super::output_block::OutputBlockCause;
use super::{ExitReason, ExitRecord, WorkerState, lock_recover, now_ms};
use crate::worker::reader::reader_lifecycle::namespace_loss::NamespaceLossDrainAdmission;
use anyhow::Result;
use std::sync::Arc;

const PROVISIONAL_LOST_PERSIST_DEFERRED_DETAIL: &str =
    "deferring Lost exit persistence until after PTY ownership cleanup";
const MISSING_EOF_LOST_PERSIST_DEFERRED_DETAIL: &str =
    "deferring missing-EOF Lost exit persistence until finalization";
const NAMESPACE_LOST_PERSIST_DEFERRED_DETAIL: &str =
    "deferring namespace-loss Lost exit persistence until finalization";

impl WorkerState {
    pub(super) fn begin_output_drain_after_namespace_loss(self: &Arc<Self>) -> bool {
        let Some(proof) = self.namespace.prove_ownership_loss() else {
            return false;
        };
        let admission = self.reader_lifecycle.begin_namespace_loss_drain(proof);
        self.activate_namespace_loss_drain(admission)
    }

    pub(super) fn begin_lost_ownership_fault(self: &Arc<Self>) {
        self.begin_system_termination_blocking(ExitReason::Lost);
        self.persist_provisional_lost(PROVISIONAL_LOST_PERSIST_DEFERRED_DETAIL);
    }

    pub(super) fn begin_lost_output_abandonment(self: &Arc<Self>) {
        self.begin_system_termination_blocking(ExitReason::Lost);
        self.mark_output_blocked(OutputBlockCause::LostOutputAbandonment);
        self.persist_provisional_lost(MISSING_EOF_LOST_PERSIST_DEFERRED_DETAIL);
    }

    fn persist_provisional_lost(&self, detail: &'static str) {
        let record = lock_recover(&self.record);
        let terminal_id = record.terminal_id;
        let runtime_epoch = record.runtime_epoch;
        drop(record);
        let exit = ExitRecord {
            runtime_epoch,
            code: None,
            signal: None,
            reason: ExitReason::Lost,
            observed_at_ms: now_ms(),
            last_output_sequence: lock_recover(&self.journal).last_sequence(),
        };
        if let Err(error) = try_persist_provisional_exit(self, terminal_id, &exit) {
            self.mark_output_blocked(OutputBlockCause::ExitPersistence);
            let error_detail = super::supervision::error_detail(error);
            let _ = super::supervision::contain(|| {
                tracing::error!(%error_detail, %terminal_id, message = detail);
            });
        }
        // Only the finalization writer clears ExitPersistence: a successful
        // provisional Lost record does not prove the final exit record is
        // durable.
    }

    fn activate_namespace_loss_drain(
        self: &Arc<Self>,
        admission: NamespaceLossDrainAdmission,
    ) -> bool {
        match admission {
            NamespaceLossDrainAdmission::Blocked => false,
            NamespaceLossDrainAdmission::Active => true,
            NamespaceLossDrainAdmission::Entered => {
                self.begin_system_termination_blocking(ExitReason::Lost);
                self.mark_output_blocked(OutputBlockCause::NamespaceOwnershipLoss);
                self.persist_provisional_lost(NAMESPACE_LOST_PERSIST_DEFERRED_DETAIL);
                true
            }
        }
    }
}

fn try_persist_provisional_exit(
    state: &WorkerState,
    terminal_id: uuid::Uuid,
    exit: &ExitRecord,
) -> Result<()> {
    #[cfg(test)]
    if state.control_test_hooks.exit_persist_is_blocked() {
        return Err(anyhow::anyhow!(
            super::control_test_hooks::INJECTED_EXIT_PERSIST_FAILURE
        ));
    }
    state.persist_json(&state.paths.exit_record(terminal_id), exit)
}
