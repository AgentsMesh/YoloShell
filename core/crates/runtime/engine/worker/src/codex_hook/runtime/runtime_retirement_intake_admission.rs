use super::super::error::CodexHookRejection;
use super::super::router::{CodexRetirementPermit, CodexRouterError};
use super::CodexHookRuntime;
use crate::agent::hooks::{EventSequence, HookEnvelope, HookViolation};
use crate::codex_hooks::{
    CodexHookIntake, CodexIntakeOccurrence, CodexRetirementSealArtifact, CodexRetirementWriterFence,
};

pub(in crate::worker::codex_hook) struct CodexRetirementIntakeAuthority<'a> {
    permit: &'a CodexRetirementPermit,
    writer: &'a CodexRetirementWriterFence,
    seal: &'a CodexRetirementSealArtifact,
}

pub(in crate::worker::codex_hook) struct VerifiedRetirementOccurrence<'a> {
    permit: &'a CodexRetirementPermit,
    writer: &'a CodexRetirementWriterFence,
    seal: &'a CodexRetirementSealArtifact,
    envelope: HookEnvelope,
}

impl<'a> CodexRetirementIntakeAuthority<'a> {
    pub(in crate::worker::codex_hook) fn new(
        runtime: &CodexHookRuntime,
        permit: &'a CodexRetirementPermit,
        writer: &'a CodexRetirementWriterFence,
        seal: &'a CodexRetirementSealArtifact,
    ) -> Result<Self, CodexHookRejection> {
        let authority = Self {
            permit,
            writer,
            seal,
        };
        verify_permitted_route(runtime, permit, writer, seal, &runtime.owner)?;
        Ok(authority)
    }

    pub(in crate::worker::codex_hook) fn authorize(
        &self,
        runtime: &CodexHookRuntime,
        intake: &CodexHookIntake,
        occurrence: &CodexIntakeOccurrence,
    ) -> Result<VerifiedRetirementOccurrence<'_>, CodexHookRejection> {
        intake
            .verify_current(occurrence)
            .map_err(storage_rejection)?;
        let envelope = occurrence.envelope().clone();
        intake
            .verify_admission(self.permit.entry().admission().authority())
            .map_err(storage_rejection)?;
        verify_permitted_route(
            runtime,
            self.permit,
            self.writer,
            self.seal,
            &envelope.identity,
        )?;
        intake
            .verify_current(occurrence)
            .map_err(storage_rejection)?;
        verify_permitted_route(
            runtime,
            self.permit,
            self.writer,
            self.seal,
            &envelope.identity,
        )?;
        Ok(VerifiedRetirementOccurrence {
            permit: self.permit,
            writer: self.writer,
            seal: self.seal,
            envelope,
        })
    }

    pub(in crate::worker::codex_hook) fn recover_receipts(
        &self,
        runtime: &CodexHookRuntime,
        intake: &CodexHookIntake,
    ) -> Result<usize, CodexHookRejection> {
        let admission = self.permit.entry().admission().authority();
        verify_permitted_route(runtime, self.permit, self.writer, self.seal, intake.owner())?;
        let removed = intake
            .recover_sealed_receipts(&runtime.outbox, self.writer, self.seal, admission)
            .map_err(storage_rejection)?;
        verify_permitted_route(runtime, self.permit, self.writer, self.seal, intake.owner())?;
        Ok(removed)
    }
}

impl VerifiedRetirementOccurrence<'_> {
    pub(in crate::worker::codex_hook) fn verify(
        &self,
        runtime: &CodexHookRuntime,
        event: &HookEnvelope,
    ) -> Result<(), CodexHookRejection> {
        event.identity.validate().map_err(reducer_rejection)?;
        let mut unassigned = event.clone();
        unassigned.sequence = EventSequence::ZERO;
        if unassigned != self.envelope {
            return Err(identity_rejection());
        }
        verify_borrowed_route(
            runtime,
            self.permit,
            self.writer,
            self.seal,
            &self.envelope.identity,
        )
    }

    pub(in crate::worker::codex_hook) fn verify_intake(
        &self,
        runtime: &CodexHookRuntime,
        intake: &CodexHookIntake,
        event: &HookEnvelope,
    ) -> Result<(), CodexHookRejection> {
        self.verify(runtime, event)?;
        intake
            .verify_admission(self.permit.entry().admission().authority())
            .map_err(storage_rejection)?;
        self.verify(runtime, event)
    }
}

fn verify_permitted_route(
    runtime: &CodexHookRuntime,
    permit: &CodexRetirementPermit,
    writer: &CodexRetirementWriterFence,
    seal: &CodexRetirementSealArtifact,
    identity: &crate::agent::hooks::HookIdentity,
) -> Result<(), CodexHookRejection> {
    permit.verify().map_err(router_rejection)?;
    verify_borrowed_route(runtime, permit, writer, seal, identity)?;
    permit.verify().map_err(router_rejection)
}

fn verify_borrowed_route(
    runtime: &CodexHookRuntime,
    permit: &CodexRetirementPermit,
    writer: &CodexRetirementWriterFence,
    seal: &CodexRetirementSealArtifact,
    identity: &crate::agent::hooks::HookIdentity,
) -> Result<(), CodexHookRejection> {
    let entry = permit.entry();
    let admission = entry.admission();
    writer
        .verify_seal(seal, admission.authority())
        .map_err(storage_rejection)?;
    if !std::ptr::eq(entry.runtime().as_ref(), runtime)
        || &runtime.owner != identity
        || &admission.record().request().identity != identity
    {
        return Err(identity_rejection());
    }
    writer
        .verify_seal(seal, admission.authority())
        .map_err(storage_rejection)
}

fn identity_rejection() -> CodexHookRejection {
    CodexHookRejection::Identity(super::super::constants::CODEX_OWNER_ERROR)
}

fn reducer_rejection(error: HookViolation) -> CodexHookRejection {
    CodexHookRejection::Violation(error)
}

fn router_rejection(error: CodexRouterError) -> CodexHookRejection {
    storage_rejection(anyhow::Error::new(error))
}

fn storage_rejection(error: anyhow::Error) -> CodexHookRejection {
    CodexHookRejection::Storage(error)
}
