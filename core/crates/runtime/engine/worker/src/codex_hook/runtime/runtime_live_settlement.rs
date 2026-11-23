use super::super::authority::LifecycleAuthority;
use super::super::constants::CODEX_STORAGE_ERROR;
use super::super::error::CodexHookRejection;
use super::live_failure::CodexLiveFailure;
use super::live_producer::CodexLiveProducerProof;
use super::{CodexHookRuntime, CodexLiveIngest, CommittedLiveEvent, VerifiedLiveOccurrence};
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::CodexRetirementWriterFence;
use anyhow::{Context, Result};

pub(in crate::worker::codex_hook) struct CodexLiveSettlement {
    event: HookEnvelope,
    occurrence: Option<VerifiedLiveOccurrence>,
    writer: CodexRetirementWriterFence,
    _generation_fence: super::super::cancellation::IngressGenerationFence,
}

impl CodexHookRuntime {
    pub(super) fn settle_live(
        &self,
        lifecycle: &mut LifecycleAuthority,
        committed: CommittedLiveEvent,
        producer: &mut CodexLiveProducerProof,
    ) -> Result<CodexLiveIngest, CodexLiveFailure> {
        let publication = self
            .ingest_committed_locked(lifecycle, &committed, || producer.verify_writer())
            .map_err(|error| CodexLiveFailure::committed(&committed, error))?;
        let event = committed.event.clone();
        let acknowledgement = match committed.occurrence.as_ref() {
            Some(occurrence) => occurrence.durable_identity().and_then(|identity| {
                self.outbox
                    .acknowledge_occurrence(&event, occurrence.envelope(), &identity)
            }),
            None => self.outbox.acknowledge(event.sequence.0, event.event_id),
        };
        if let Err(error) = acknowledgement.context(CODEX_STORAGE_ERROR) {
            let rejection = classify_outbox_error(error);
            if rejection.is_commit_indeterminate() {
                self.latch_commit_indeterminate_locked(lifecycle);
            }
            return Err(CodexLiveFailure::committed(&committed, rejection));
        }
        if let Err(error) = producer.verify_writer() {
            return Err(CodexLiveFailure::committed(&committed, error));
        }
        let writer = match producer.take_retirement_writer() {
            Ok(writer) => writer,
            Err(error) => return Err(CodexLiveFailure::committed(&committed, error)),
        };
        let generation_fence = committed.generation_fence;
        Ok(CodexLiveIngest {
            event: event.clone(),
            publication: publication.publication,
            rejection: publication.rejection,
            settlement: CodexLiveSettlement {
                event,
                occurrence: committed.occurrence,
                writer,
                _generation_fence: generation_fence,
            },
        })
    }

    pub(in crate::worker::codex_hook) fn complete_live_settlement(
        &self,
        settlement: CodexLiveSettlement,
    ) -> Result<()> {
        settlement.complete(&self.outbox)
    }
}

fn classify_outbox_error(error: anyhow::Error) -> CodexHookRejection {
    if crate::codex_hooks::is_outbox_commit_indeterminate(&error) {
        CodexHookRejection::CommitIndeterminate(error)
    } else {
        CodexHookRejection::Storage(error)
    }
}

impl CodexLiveSettlement {
    fn complete(self, outbox: &crate::codex_hooks::CodexOutbox) -> Result<()> {
        let writer = self.writer;
        writer.verify_unsealed().context(CODEX_STORAGE_ERROR)?;
        let Some(occurrence) = self.occurrence else {
            return Ok(());
        };
        let raw = occurrence.envelope().clone();
        let identity = occurrence.durable_identity()?;
        occurrence.complete_with_fence(|| writer.verify_unsealed())?;
        writer.verify_unsealed().context(CODEX_STORAGE_ERROR)?;
        outbox
            .remove_occurrence_receipt(&self.event, &raw, &identity)
            .context(CODEX_STORAGE_ERROR)?;
        writer.verify_unsealed().context(CODEX_STORAGE_ERROR)
    }
}
