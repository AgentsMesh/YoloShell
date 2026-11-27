use super::super::error::CodexHookRejection;
use super::super::router::DurableCodexAdmission;
use super::{CodexHookRuntime, live_occurrence::VerifiedLiveOccurrence};
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::{
    CodexEnqueueAdmission, CodexExecutableKind::Runtime, CodexRetirementWriterFence,
};

#[path = "runtime_live_validation.rs"]
mod validation;
#[path = "runtime_live_writer.rs"]
mod writer;
use validation::{durable_rejection, process_rejection, storage_rejection, verify_occurrence};

#[derive(Clone)]
pub(in crate::worker::codex_hook) struct CodexLiveProducer {
    peer: crate::process::ProcessIdentity,
    admission: DurableCodexAdmission,
    generation: super::super::cancellation::IngressCancellation,
}

pub(super) struct CodexLiveProducerProof {
    producer: CodexLiveProducer,
    writer: Option<CodexRetirementWriterFence>,
}

impl CodexLiveProducer {
    pub(in crate::worker::codex_hook) fn new(
        peer: crate::process::ProcessIdentity,
        admission: DurableCodexAdmission,
        generation: super::super::cancellation::IngressCancellation,
    ) -> Self {
        Self {
            peer,
            admission,
            generation,
        }
    }

    pub(super) fn preflight(
        self,
        runtime: &CodexHookRuntime,
        event: &HookEnvelope,
    ) -> Result<CodexLiveProducerProof, CodexHookRejection> {
        self.verify_generation()?;
        runtime.validate_event_scope(event)?;
        self.admission.verify().map_err(storage_rejection)?;
        let record = self.admission.record();
        if record.request().identity != event.identity {
            return Err(CodexHookRejection::Identity(
                super::super::constants::CODEX_OWNER_ERROR,
            ));
        }
        let executable = record.hook_executable();
        executable
            .verify_current(Runtime)
            .map_err(process_rejection)?;
        let process = crate::process::ProcessIdentity {
            pid: event.identity.process.pid,
            start_time: event.identity.process.start_time,
        };
        let admitted = runtime.admit_record_process(process, self.peer, record);
        self.verify_generation()?;
        admitted?;
        executable
            .verify_current(Runtime)
            .map_err(process_rejection)?;
        self.admission.verify().map_err(storage_rejection)?;
        self.verify_generation()?;
        let writer = self
            .admission
            .authority()
            .acquire_retirement_writer_fence()
            .map_err(CodexHookRejection::Storage)?;
        writer
            .verify_unsealed()
            .map_err(CodexHookRejection::Storage)?;
        Ok(CodexLiveProducerProof {
            producer: self,
            writer: Some(writer),
        })
    }

    pub(in crate::worker::codex_hook) fn generation(
        &self,
    ) -> super::super::cancellation::IngressCancellation {
        self.generation.clone()
    }

    pub(super) fn acquire_generation_fence(
        &self,
    ) -> Result<super::super::cancellation::IngressGenerationFence, CodexHookRejection> {
        self.generation
            .try_fence()
            .ok_or(CodexHookRejection::Superseded)
    }

    fn verify_generation(&self) -> Result<(), CodexHookRejection> {
        if self.generation.is_cancelled() {
            return Err(CodexHookRejection::Superseded);
        }
        Ok(())
    }
}

impl CodexLiveProducerProof {
    pub(super) fn authenticate(
        &self,
        runtime: &CodexHookRuntime,
        event: &HookEnvelope,
        admission: CodexEnqueueAdmission,
    ) -> Result<Option<VerifiedLiveOccurrence>, CodexHookRejection> {
        self.producer.verify_generation()?;
        runtime.validate_event_scope(event)?;
        self.producer
            .admission
            .verify()
            .map_err(storage_rejection)?;
        let record = self.producer.admission.record();
        if record.request().identity != event.identity {
            return Err(CodexHookRejection::Identity(
                super::super::constants::CODEX_OWNER_ERROR,
            ));
        }
        let executable = record.hook_executable();
        executable
            .verify_current(Runtime)
            .map_err(process_rejection)?;
        let occurrence = match admission {
            CodexEnqueueAdmission::New => {
                Some(VerifiedLiveOccurrence::required(runtime, event).map_err(durable_rejection)?)
            }
            CodexEnqueueAdmission::Replay => {
                VerifiedLiveOccurrence::optional(runtime, event).map_err(durable_rejection)?
            }
        };
        self.verify_commit(runtime, event, admission, occurrence.as_ref())?;
        Ok(occurrence)
    }

    pub(super) fn verify_commit(
        &self,
        runtime: &CodexHookRuntime,
        event: &HookEnvelope,
        admission: CodexEnqueueAdmission,
        occurrence: Option<&VerifiedLiveOccurrence>,
    ) -> Result<(), CodexHookRejection> {
        self.producer.verify_generation()?;
        if matches!(admission, CodexEnqueueAdmission::New) && occurrence.is_none() {
            return Err(CodexHookRejection::Storage(anyhow::anyhow!(
                super::super::constants::CODEX_DURABLE_EVENT_ERROR
            )));
        }
        runtime.validate_event_scope(event)?;
        self.producer
            .admission
            .verify()
            .map_err(storage_rejection)?;
        let record = self.producer.admission.record();
        if record.request().identity != event.identity {
            return Err(CodexHookRejection::Identity(
                super::super::constants::CODEX_OWNER_ERROR,
            ));
        }
        let executable = record.hook_executable();
        executable
            .verify_current(Runtime)
            .map_err(process_rejection)?;
        verify_occurrence(occurrence)?;
        executable
            .verify_current(Runtime)
            .map_err(process_rejection)?;
        self.producer
            .admission
            .verify()
            .map_err(storage_rejection)?;
        verify_occurrence(occurrence)?;
        self.producer.verify_generation()?;
        Ok(())
    }

    pub(super) fn verify_generation(&self) -> Result<(), CodexHookRejection> {
        self.producer.verify_generation()
    }

    pub(super) fn acquire_generation_fence(
        &self,
    ) -> Result<super::super::cancellation::IngressGenerationFence, CodexHookRejection> {
        self.producer.acquire_generation_fence()
    }
}
