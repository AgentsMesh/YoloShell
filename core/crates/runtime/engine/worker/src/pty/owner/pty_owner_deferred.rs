use super::super::output_block::OutputBlockCause;
use super::super::pty_fault_reaper::FaultReapCompletion;
use super::super::pty_owner_cleanup::{RootExitCleanup, cleanup_after_root_exit};
use super::super::pty_ownership::FaultSettlement;
use super::super::termination_timing::CapabilitySettlementDeferred;
use super::{PtyOwner, PtyOwnerCommandFailure};
use crate::process::UnreapedRootExit;
use std::sync::Arc;

impl PtyOwner {
    pub(crate) fn finish_termination_error(&mut self, error: anyhow::Error) {
        match error.downcast::<CapabilitySettlementDeferred>() {
            Ok(deferred) => self.finish_cleanup_deferred(deferred),
            Err(error) => self.finish_ownership_fault(error),
        }
    }

    pub(crate) fn finish_cleanup_deferred(&mut self, deferred: CapabilitySettlementDeferred) {
        let state = self.state.upgrade();
        self.complete_commands_with(Err(PtyOwnerCommandFailure::CleanupDeferred(deferred)));
        if let Some(state) = state.as_ref()
            && !state.has_termination_intent()
        {
            state.mark_output_blocked(OutputBlockCause::DeferredCleanup);
        }
        let completion = state
            .as_ref()
            .map(|state| FaultReapCompletion::finalize(Arc::clone(state)))
            .unwrap_or_else(FaultReapCompletion::log);
        match self.ownership.settle_after_fault(completion) {
            FaultSettlement::Reaped(outcome) => {
                self.ownership.disarm();
                if let Some(state) = state {
                    super::complete_deferred_reap(state, outcome);
                }
            }
            FaultSettlement::Deferred(_) => {}
        }
    }

    pub(crate) fn finish_root_exit(&mut self, proof: UnreapedRootExit) {
        if let Err(error) = self.ownership.mark_root_exited_unreaped(proof) {
            self.finish_ownership_fault(error);
            return;
        }
        if let Err(error) = self.ownership.wait_root_exited() {
            self.finish_ownership_fault(error);
            return;
        }
        if let Some(state) = self.state.upgrade() {
            state.publish_direct_child_waited();
        }
        if self.ownership.session_is_retired() {
            self.reap_complete_and_finalize();
            return;
        }
        match cleanup_after_root_exit(&mut self.ownership, &mut self.output_recovery) {
            Ok(RootExitCleanup::Complete) => self.reap_complete_and_finalize(),
            Ok(RootExitCleanup::Lost) => {
                if let Some(state) = self.state.upgrade() {
                    state.begin_lost_output_abandonment();
                }
                self.reap_complete_and_finalize();
            }
            Ok(RootExitCleanup::Deferred(deferred)) => self.finish_cleanup_deferred(deferred),
            Err(error) => self.finish_ownership_fault(error),
        }
    }
}
