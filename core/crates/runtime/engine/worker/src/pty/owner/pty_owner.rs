use super::WorkerState;
use super::constants::PROCESS_SCOPE_REFRESH_INTERVAL;
use super::pty_exit_observer::RootExitObserver;
use super::pty_output_progress::PtyOutputRecovery;
use super::pty_owner_command_channel::PtyOwnerCommandReceiver;
use super::pty_owner_handle::{
    CommandCompletion, OwnerSettlement, OwnerSettlementPublisher, PtyOwnerCommand,
    PtyOwnerCommandFailure,
};
use super::pty_owner_termination::OwnerTermination;
use super::pty_ownership::PtyOwnership;
use super::supervision;
use super::terminal_reporting;
use super::termination_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
use anyhow::{Result, anyhow};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Weak, mpsc};
use std::time::Instant;
use tokio::sync::oneshot;

pub(crate) const PTY_OWNER_FAULT_DETAIL: &str = "PTY lifetime owner entered an ownership fault";
pub(crate) const PTY_OWNER_PANIC_DETAIL: &str = "PTY lifetime owner panicked";
pub(crate) const PTY_OWNER_RECOVERY_PANIC_DETAIL: &str =
    "PTY lifetime owner fault recovery panicked";
pub(crate) const PTY_OWNER_CHANNEL_CLOSED_DETAIL: &str =
    "PTY lifetime owner command channel disconnected";
#[path = "pty_owner_completion.rs"]
mod completion;
#[path = "pty_owner_custody.rs"]
mod custody;
#[path = "pty_owner_deferred.rs"]
mod deferred;
pub(crate) use completion::complete_deferred_reap;
pub(crate) use custody::PtyOwnerCustody;
pub(crate) struct PtyOwner {
    pub(crate) state: Weak<WorkerState>,
    pub(crate) ownership: PtyOwnership,
    pub(crate) output_recovery: PtyOutputRecovery,
    pub(crate) observer: Box<dyn RootExitObserver>,
    pub(crate) commands: Option<PtyOwnerCommandReceiver>,
    pub(crate) pending: Vec<oneshot::Sender<CommandCompletion>>,
    pub(crate) termination: Option<OwnerTermination>,
    pub(crate) failure: Option<anyhow::Error>,
    pub(crate) next_refresh: Instant,
    pub(crate) settlement: OwnerSettlement,
    pub(crate) _settlement_publisher: OwnerSettlementPublisher,
}
impl PtyOwner {
    pub(crate) fn run(mut self) {
        if let Err(payload) = catch_unwind(AssertUnwindSafe(|| self.run_owned())) {
            let detail = supervision::panic_detail(payload);
            let recovered = catch_unwind(AssertUnwindSafe(|| {
                self.finish_ownership_fault(anyhow!("{PTY_OWNER_PANIC_DETAIL}: {detail}"));
            }));
            if let Err(payload) = recovered {
                let detail = supervision::panic_detail(payload);
                terminal_reporting::error_detail(&detail, PTY_OWNER_RECOVERY_PANIC_DETAIL);
            }
        }
        let _ = supervision::contain(|| drop(self.failure.take()));
    }

    pub(crate) fn run_owned(&mut self) {
        loop {
            match self.ownership.poll_root_exit(self.observer.as_mut()) {
                Ok(Some(proof)) => {
                    self.finish_root_exit(proof);
                    return;
                }
                Ok(None) => {}
                Err(error) => {
                    self.finish_ownership_fault(error);
                    return;
                }
            }
            let received = self
                .commands
                .as_ref()
                .unwrap()
                .recv_timeout(GRACEFUL_TERMINATION_POLL_INTERVAL);
            match received {
                Ok(command) => self.accept(command),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.finish_ownership_fault(anyhow!(PTY_OWNER_CHANNEL_CLOSED_DETAIL));
                    return;
                }
            }
            if let Err(error) = self.refresh_tracking() {
                self.finish_ownership_fault(error);
                return;
            }
            if let Err(error) = self.drive_termination() {
                self.finish_termination_error(error);
                return;
            }
        }
    }
    pub(crate) fn refresh_tracking(&mut self) -> Result<()> {
        let now = Instant::now();
        if now < self.next_refresh {
            return Ok(());
        }
        self.next_refresh = now + PROCESS_SCOPE_REFRESH_INTERVAL;
        let Some(tree) = self.ownership.tracking_tree_mut() else {
            return Ok(());
        };
        tree.refresh_if_changed()?;
        if self.ownership.root_exit_requires_output_progress()? {
            self.output_recovery.progress()?;
        }
        Ok(())
    }
    pub(crate) fn accept(&mut self, command: PtyOwnerCommand) {
        match command {
            PtyOwnerCommand::Terminate { force, response } => {
                self.ownership.record_force_request(force);
                self.termination
                    .get_or_insert_with(OwnerTermination::new)
                    .request(force);
                self.pending.push(response);
            }
        }
    }

    pub(crate) fn drive_termination(&mut self) -> Result<()> {
        let Some(termination) = self.termination.as_mut() else {
            return Ok(());
        };
        #[cfg(feature = "termination-test-hooks")]
        if let Some(state) = self.state.upgrade() {
            if super::termination_test_hook::owner_is_held(state.terminal_id())? {
                return Ok(());
            }
        }
        match termination.poll(&mut self.ownership, &mut self.output_recovery)? {
            super::pty_force_termination::ForceTerminationPoll::Unsupported(proof) => {
                self.ownership.enter_observer_only();
                Self::complete_unsupported(&mut self.pending, proof);
            }
            super::pty_force_termination::ForceTerminationPoll::Complete => {
                self.complete_commands();
            }
            super::pty_force_termination::ForceTerminationPoll::Pending => {}
        }
        Ok(())
    }

    pub(crate) fn complete_unsupported(
        pending: &mut Vec<oneshot::Sender<CommandCompletion>>,
        proof: crate::process::DestructiveSessionUnavailable,
    ) {
        for response in pending.drain(..) {
            let failure = PtyOwnerCommandFailure::DestructiveUnavailable(proof.clone());
            let _ = response.send(Err(failure));
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/owner/pty_owner_unsupported_tests.rs"]
mod unsupported_tests;
