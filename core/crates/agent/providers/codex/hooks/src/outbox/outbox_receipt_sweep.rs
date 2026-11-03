use super::constants::errors;
use super::lock::OutboxLock;
use super::{CodexHookIntake, CodexOutbox, outbox_storage, outbox_validation};
use anyhow::{Context, Result, ensure};

impl CodexOutbox {
    pub(super) fn sweep_occurrence_receipts(
        &self,
        intake: &CodexHookIntake,
        mut verify_fence: impl FnMut() -> Result<()>,
    ) -> Result<usize> {
        verify_fence()?;
        let lock = OutboxLock::acquire(&self.directory, &self.path)?;
        let mut loaded = outbox_storage::load(&lock, &self.path)?;
        outbox_validation::check_configured(&loaded.state, self.owner.as_ref())?;
        ensure!(
            loaded.state.occurrence_receipts.is_empty()
                || loaded.state.owner.as_ref() == Some(intake.owner()),
            errors::OUTBOX_OWNER
        );
        verify_fence()?;
        let initial = loaded.state.occurrence_receipts.len();
        let receipts = std::mem::take(&mut loaded.state.occurrence_receipts);
        for receipt in receipts {
            let file_name = receipt.file_name();
            let artifact = intake
                .read_receipt_artifact(&file_name)
                .context(errors::OUTBOX_CORRUPT)?;
            let artifact = match artifact {
                super::intake_lookup::CodexReceiptArtifact::Missing => {
                    verify_fence()?;
                    intake.directory().sync().context(errors::OUTBOX_CORRUPT)?;
                    intake
                        .directory()
                        .verify_path()
                        .context(errors::OUTBOX_CORRUPT)?;
                    verify_fence()?;
                    intake
                        .read_receipt_artifact(&file_name)
                        .context(errors::OUTBOX_CORRUPT)?
                }
                artifact => artifact,
            };
            match artifact {
                super::intake_lookup::CodexReceiptArtifact::Active(envelope, identity) => {
                    receipt.assign(&envelope, &identity)?;
                    loaded.state.occurrence_receipts.push_back(receipt);
                }
                super::intake_lookup::CodexReceiptArtifact::Isolated(envelope, identity) => {
                    receipt.assign(&envelope, &identity)?;
                    verify_fence()?;
                    intake.complete_receipt_artifact(&identity)?;
                    verify_fence()?;
                }
                super::intake_lookup::CodexReceiptArtifact::Missing => {}
            }
        }
        verify_fence()?;
        let removed = initial
            .checked_sub(loaded.state.occurrence_receipts.len())
            .context(errors::OUTBOX_CORRUPT)?;
        if removed == usize::default() {
            outbox_storage::run_after_operation_hook();
            outbox_storage::verify_unchanged(&lock, &self.path, &loaded)?;
        } else {
            outbox_storage::persist_with_commit_fence(&lock, &self.path, &loaded, || {
                verify_fence()
            })?;
        }
        verify_fence()?;
        Ok(removed)
    }
}
