use super::super::error::CodexHookRejection;
use super::super::router::{CodexRouteLease, CodexRouterError, CodexSessionRouter};
use super::CodexHookRuntime;
use crate::agent::hooks::{EventSequence, HookEnvelope, HookViolation};
use crate::codex_hooks::{CodexHookIntake, CodexIntakeOccurrence, CodexRetirementWriterFence};

pub(in crate::worker::codex_hook) struct VerifiedDurableOccurrence {
    route: CodexRouteLease,
    writer: CodexRetirementWriterFence,
    envelope: HookEnvelope,
}

impl VerifiedDurableOccurrence {
    pub(in crate::worker::codex_hook) fn authorize(
        runtime: &CodexHookRuntime,
        router: &CodexSessionRouter,
        intake: &CodexHookIntake,
        occurrence: &CodexIntakeOccurrence,
    ) -> Result<Self, CodexHookRejection> {
        intake
            .verify_current(occurrence)
            .map_err(storage_rejection)?;
        let identity = occurrence.envelope().identity.clone();
        let route = router
            .lease_committed(&identity)
            .map_err(router_rejection)?;
        let writer = route
            .entry()
            .admission()
            .authority()
            .acquire_retirement_writer_fence()
            .map_err(storage_rejection)?;
        writer.verify_unsealed().map_err(storage_rejection)?;
        intake
            .verify_admission(route.entry().admission().authority())
            .map_err(storage_rejection)?;
        intake
            .verify_current(occurrence)
            .map_err(storage_rejection)?;
        route
            .entry()
            .admission()
            .verify()
            .map_err(router_rejection)?;
        writer.verify_unsealed().map_err(storage_rejection)?;
        let proof = Self {
            route,
            writer,
            envelope: occurrence.envelope().clone(),
        };
        proof.verify_route(runtime)?;
        Ok(proof)
    }

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
        self.verify_route(runtime)
    }

    pub(in crate::worker::codex_hook) fn verify_intake(
        &self,
        runtime: &CodexHookRuntime,
        intake: &CodexHookIntake,
        event: &HookEnvelope,
    ) -> Result<(), CodexHookRejection> {
        self.verify(runtime, event)?;
        intake
            .verify_admission(self.route.entry().admission().authority())
            .map_err(storage_rejection)?;
        self.verify(runtime, event)
    }

    fn verify_route(&self, runtime: &CodexHookRuntime) -> Result<(), CodexHookRejection> {
        self.writer.verify_unsealed().map_err(storage_rejection)?;
        let entry = self.route.entry();
        entry.admission().verify().map_err(router_rejection)?;
        if !std::ptr::eq(entry.runtime().as_ref(), runtime)
            || self.envelope.identity != runtime.owner
            || entry.admission().record().request().identity != self.envelope.identity
        {
            return Err(identity_rejection());
        }
        self.writer.verify_unsealed().map_err(storage_rejection)?;
        entry.admission().verify().map_err(router_rejection)?;
        Ok(())
    }
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
