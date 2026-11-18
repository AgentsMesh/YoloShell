use super::super::output_block::OutputBlockCause;
use super::super::pty_fault_reaper::FaultReapCompletion;
use super::super::pty_ownership::FaultSettlement;
use super::super::pty_ownership::{ReapEvidence, ReapOutcome};
use super::super::{finalize_lost_unknown_child, finalize_waited_child};
use super::WorkerState;
use super::{CommandCompletion, PtyOwner, PtyOwnerCommandFailure};
use crate::model::ExitReason;
use anyhow::Result;
use std::sync::Arc;

pub(crate) const OWNER_FAILURE_MISSING_DETAIL: &str =
    "PTY owner fault is present after command settlement";

impl PtyOwner {
    pub(crate) fn finish_ownership_fault(&mut self, error: anyhow::Error) {
        let state = self.state.upgrade();
        if let Some(state) = state.as_ref() {
            state.publish_owner_fault(&error);
        }
        self.record(Err(error));
        self.complete_commands();
        if let Some(state) = state.as_ref() {
            let _ = super::super::supervision::contain(|| {
                state.begin_lost_ownership_fault();
                #[cfg(test)]
                state
                    .control_test_hooks
                    .owner_fault_before_cleanup
                    .pause_if_armed();
            });
        }
        let fault = self.failure.take().expect(OWNER_FAILURE_MISSING_DETAIL);
        let reap_completion = state
            .as_ref()
            .map(|state| FaultReapCompletion::finalize(Arc::clone(state)))
            .unwrap_or_else(FaultReapCompletion::log);
        match self
            .ownership
            .settle_after_fault_with_failure(reap_completion, fault)
        {
            FaultSettlement::Reaped(outcome) => self.complete_reaped_and_finalize(outcome),
            FaultSettlement::Deferred(error) => {
                self.record(Err(error));
            }
        }
    }

    pub(crate) fn reap_complete_and_finalize(&mut self) {
        let outcome = self.ownership.wait_until_reaped();
        self.complete_reaped_and_finalize(outcome);
    }

    pub(crate) fn complete_reaped_and_finalize(&mut self, mut outcome: ReapOutcome) {
        if let Some(error) = outcome.fault.take() {
            self.record(Err(error));
        }
        self.complete_commands();
        self.ownership.disarm();
        let Some(state) = self.state.upgrade() else {
            return;
        };
        if self.failure.is_some() {
            state.begin_system_termination_blocking(ExitReason::Lost);
        }
        finalize_outcome(&state, &outcome);
    }

    pub(crate) fn complete_commands(&mut self) {
        let completion = self.failure.as_ref().map_or(Ok(()), |error| {
            Err(PtyOwnerCommandFailure::Owner(
                super::super::supervision::error_ref_detail(error),
            ))
        });
        self.complete_commands_with(completion);
    }

    pub(crate) fn complete_commands_with(&mut self, completion: CommandCompletion) {
        self.complete_commands_before_publication(completion, || {});
    }

    #[cfg(test)]
    pub(crate) fn complete_commands_before_publication_for_test(
        &mut self,
        completion: CommandCompletion,
        before_publication: impl FnOnce(),
    ) {
        self.complete_commands_before_publication(completion, before_publication);
    }

    pub(crate) fn complete_commands_before_publication(
        &mut self,
        completion: CommandCompletion,
        before_publication: impl FnOnce(),
    ) {
        let completion = self.settlement.get().cloned().unwrap_or(completion);
        if let Some(commands) = self.commands.take() {
            for command in commands.close_and_drain() {
                match command {
                    super::PtyOwnerCommand::Terminate { force, response } => {
                        self.ownership.record_force_request(force);
                        self.pending.push(response);
                    }
                }
            }
        }
        for response in self.pending.drain(..) {
            let _ = response.send(completion.clone());
        }
        before_publication();
        let _ = self.settlement.set(completion);
    }

    pub(crate) fn record(&mut self, result: Result<()>) {
        let Err(error) = result else { return };
        if self.failure.is_none() {
            let detail = super::super::supervision::error_ref_detail(&error);
            super::super::terminal_reporting::error_detail(&detail, super::PTY_OWNER_FAULT_DETAIL);
            self.failure = Some(error);
        } else {
            super::super::terminal_reporting::discard_error(error);
        }
    }
}

pub(crate) fn complete_deferred_reap(state: Arc<WorkerState>, mut outcome: ReapOutcome) {
    state.clear_output_blocked(OutputBlockCause::DeferredCleanup);
    if let Some(error) = outcome.fault.as_ref() {
        let _ = super::super::supervision::contain(|| {
            let detail = super::super::supervision::error_ref_detail(error);
            super::super::terminal_reporting::error_detail(&detail, super::PTY_OWNER_FAULT_DETAIL);
        });
    }
    let fault = outcome.fault.take();
    let _ = super::super::supervision::contain(|| drop(fault));
    let _ = super::super::supervision::contain(|| finalize_outcome(&state, &outcome));
    let _ = super::super::supervision::contain(|| drop(outcome));
}

pub(crate) fn finalize_outcome(state: &Arc<WorkerState>, outcome: &ReapOutcome) {
    if outcome.evidence == ReapEvidence::LostUnknown {
        finalize_lost_unknown_child(state);
        return;
    }
    let code = outcome
        .status
        .and_then(crate::process::DirectChildExitStatus::code);
    let signal = outcome
        .status
        .and_then(crate::process::DirectChildExitStatus::signal);
    finalize_waited_child(state, code, signal);
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/owner/pty_owner_completion_idempotency_tests.rs"]
mod idempotency_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/owner/pty_owner_deferred_completion_tests.rs"]
mod tests;
