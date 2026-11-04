use super::ReaderLifecycle;
use anyhow::{Context, Result};
use portable_pty::MasterPty;
use std::fs::File;
use std::sync::Arc;
#[path = "pty_output_journal.rs"]
mod journal;
use journal::{ExclusiveJournalOutput, SharedJournal};
#[path = "pty_output_activation.rs"]
mod activation;
#[path = "pty_output_child_wait.rs"]
mod child_wait;
#[path = "pty_output_release.rs"]
mod release;
pub(crate) use release::MasterAliasReleasePoll;
#[path = "pty_output_startup_custody.rs"]
mod startup_custody;
use startup_custody::StartupCustodyReleaseState;
#[path = "pty_output_shared.rs"]
mod shared;
use shared::SharedOutputDescriptor;
#[path = "pty_output_eof.rs"]
mod eof;
pub(crate) use eof::{
    CommittedKernelEofSealEpoch, KernelEofCommitOutcome, KernelEofValidation,
    KernelEofValidationRequest, KernelPtyEofAuthority, KernelPtyEofIdentity,
};
#[path = "pty_output_drain.rs"]
mod drain;
#[path = "pty_output_missing_eof.rs"]
mod missing_eof;
#[path = "pty_output_recovery_progress.rs"]
mod recovery_progress;
pub(crate) use drain::{
    ExclusiveOutputQuiescenceFence, OutputQuiescenceFence, OutputQuiescenceState,
};
#[path = "pty_output_states.rs"]
mod states;
pub(crate) use states::{OutputProgressState, OutputQueueState};
pub(crate) const STARTUP_JOURNAL_UNAVAILABLE_CONTEXT: &str =
    "startup PTY journal authority is unavailable";
pub(crate) struct ExclusivePtyOutput {
    pub(crate) descriptor: OutputDescriptor,
    pub(crate) startup_journal: Option<ExclusiveJournalOutput>,
    pub(crate) kernel_eof: Option<Arc<KernelPtyEofIdentity>>,
    pub(crate) kernel_eof_seal_epoch: Option<CommittedKernelEofSealEpoch>,
    pub(crate) kernel_eof_committed: Option<Arc<KernelPtyEofIdentity>>,
    pub(crate) owner_master_released: bool,
    pub(crate) output_abandoned: bool,
    pub(crate) master_alias_authority: Option<Arc<crate::process::PtyMasterAliasReleaseAuthority>>,
    pub(crate) startup_custody: StartupCustodyReleaseState,
}

pub(crate) struct ReaderBackedPtyOutput {
    pub(crate) descriptor: Arc<SharedOutputDescriptor>,
    pub(crate) reader_lifecycle: Option<Arc<ReaderLifecycle>>,
    pub(crate) child_waited: Option<Arc<std::sync::atomic::AtomicBool>>,
    pub(crate) master_alias_authority: Option<Arc<crate::process::PtyMasterAliasReleaseAuthority>>,
}

pub(crate) struct PtyOutputRecovery {
    pub(crate) descriptor: Arc<SharedOutputDescriptor>,
    pub(crate) reader_lifecycle: Option<Arc<ReaderLifecycle>>,
    pub(crate) child_waited: Option<Arc<std::sync::atomic::AtomicBool>>,
    pub(crate) master_alias_authority: Option<Arc<crate::process::PtyMasterAliasReleaseAuthority>>,
}

pub(crate) enum OutputDescriptor {
    Native(File),
    Released,
    #[cfg(test)]
    Observed(Arc<std::sync::atomic::AtomicUsize>),
    #[cfg(test)]
    Quiescent(Arc<std::sync::atomic::AtomicUsize>),
}

impl ExclusivePtyOutput {
    pub(crate) fn report_tracked_producers_gone(
        &mut self,
        _proof: &crate::process::TrackedProducersGone,
    ) -> Result<()> {
        Ok(())
    }

    pub(crate) fn mark_untracked_producers_gone(
        &mut self,
        _proof: super::startup_untracked_proof::UntrackedStartupProducersGone,
    ) -> Result<OutputProgressState> {
        self.progress()
    }

    pub(crate) fn capture(master: &dyn MasterPty, journal: SharedJournal) -> Result<Self> {
        Ok(Self {
            descriptor: super::pty_output_terminal::capture(master)?,
            startup_journal: Some(ExclusiveJournalOutput::new(journal)),
            kernel_eof: None,
            kernel_eof_seal_epoch: None,
            kernel_eof_committed: None,
            owner_master_released: false,
            output_abandoned: false,
            master_alias_authority: None,
            startup_custody: StartupCustodyReleaseState::NoSecondaryCustody,
        })
    }

    pub(crate) fn progress(&mut self) -> Result<OutputProgressState> {
        if self.output_abandoned {
            return Ok(OutputProgressState::Abandoned);
        }
        if self.kernel_eof_committed.is_some() {
            return Ok(OutputProgressState::Complete);
        }
        self.revoke_kernel_eof_authority();
        let file = match &self.descriptor {
            OutputDescriptor::Native(file) => file,
            OutputDescriptor::Released => return Ok(OutputProgressState::Complete),
            #[cfg(test)]
            OutputDescriptor::Observed(observed) => {
                observed.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                return Ok(OutputProgressState::Complete);
            }
            #[cfg(test)]
            OutputDescriptor::Quiescent(observed) => {
                observed.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                return Ok(OutputProgressState::Quiescent);
            }
        };
        super::pty_output_terminal::resume(file)?;
        let progress = self
            .startup_journal
            .as_mut()
            .context(STARTUP_JOURNAL_UNAVAILABLE_CONTEXT)?
            .progress_observing_reads(file, || {})?;
        self.record_kernel_eof_progress(progress);
        Ok(progress)
    }
}

impl ReaderBackedPtyOutput {
    pub(crate) fn recovery(&self) -> PtyOutputRecovery {
        PtyOutputRecovery {
            descriptor: Arc::clone(&self.descriptor),
            reader_lifecycle: self.reader_lifecycle.clone(),
            child_waited: self.child_waited.clone(),
            master_alias_authority: self.master_alias_authority.clone(),
        }
    }

    pub(crate) fn after_master_release() -> Self {
        Self {
            descriptor: Arc::new(SharedOutputDescriptor::new(
                OutputDescriptor::Released,
                None,
            )),
            reader_lifecycle: None,
            child_waited: None,
            master_alias_authority: None,
        }
    }
}

#[cfg(all(test, feature = "yolo-test-support"))]
include!("../../../test_support/pty_output_progress.rs");
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
include!("../../../integration_tests/pty/output/pty_output_progress_test_wiring.rs");
