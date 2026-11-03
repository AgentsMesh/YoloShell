use super::super::authority::LifecycleAuthority;
use super::super::error::CodexHookRejection;
use super::live_failure::CodexLiveFailure;
use super::live_producer::CodexLiveProducerProof;
use super::{
    CodexHookPublication, CodexHookRuntime, CodexLiveProducer, CodexLiveSettlement,
    CommittedLiveEvent, VerifiedLiveOccurrence,
};
use crate::agent::hooks::{ApplyDisposition, HookEnvelope, HookViolation};
use anyhow::ensure;

pub(in crate::worker::codex_hook) struct CodexLiveIngest {
    pub(in crate::worker::codex_hook) event: HookEnvelope,
    pub(in crate::worker::codex_hook) publication: CodexHookPublication,
    pub(in crate::worker::codex_hook) rejection: Option<HookViolation>,
    pub(in crate::worker::codex_hook) settlement: CodexLiveSettlement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::worker::codex_hook) enum CodexLiveHandoffOutcome {
    Completed,
    CommitIndeterminate,
    Deferred,
}

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn ingest_live_with_handoff(
        &self,
        occurrence: HookEnvelope,
        producer: CodexLiveProducer,
        handoff: impl FnOnce(Result<CodexLiveIngest, CodexLiveFailure>) -> CodexLiveHandoffOutcome,
    ) {
        if self.commit_indeterminate_latched() {
            let _ = handoff(Err(CodexLiveFailure::latched_commit_indeterminate()));
            return;
        }
        let mut producer = match producer.preflight(self, &occurrence) {
            Ok(producer) => producer,
            Err(error) => {
                let failure = CodexLiveFailure::after_preflight(self, &occurrence, error);
                let _ = handoff(Err(failure));
                return;
            }
        };
        let mut lifecycle = match self.lock_lifecycle() {
            Ok(lifecycle) => lifecycle,
            Err(error) => {
                let failure = CodexLiveFailure::after_preflight(self, &occurrence, error);
                let _ = handoff(Err(failure));
                return;
            }
        };
        if lifecycle.commit_indeterminate() {
            let _ = handoff(Err(CodexLiveFailure::latched_commit_indeterminate()));
            return;
        }
        let result = self.ingest_live_locked(&mut lifecycle, occurrence, &mut producer);
        if matches!(
            handoff(result),
            CodexLiveHandoffOutcome::CommitIndeterminate
        ) {
            self.latch_commit_indeterminate_locked(&mut lifecycle);
        }
    }

    fn commit_indeterminate_latched(&self) -> bool {
        self.lock_lifecycle()
            .map(|lifecycle| lifecycle.commit_indeterminate())
            .unwrap_or(false)
    }

    fn ingest_live_locked(
        &self,
        lifecycle: &mut LifecycleAuthority,
        occurrence: HookEnvelope,
        producer: &mut CodexLiveProducerProof,
    ) -> Result<CodexLiveIngest, CodexLiveFailure> {
        let authoritative_occurrence = occurrence.clone();
        let committed = match self.enqueue_live(lifecycle, occurrence, producer) {
            Ok(committed) => committed,
            Err(error) if error.is_commit_indeterminate() => {
                self.latch_commit_indeterminate_locked(lifecycle);
                return Err(CodexLiveFailure::uncommitted(error));
            }
            Err(error) => {
                return Err(CodexLiveFailure::after_preflight(
                    self,
                    &authoritative_occurrence,
                    error,
                ));
            }
        };
        #[cfg(test)]
        self.wait_for_commit_settlement_gate_for_test();
        self.settle_live(lifecycle, committed, producer)
    }

    pub(super) fn latch_commit_indeterminate_locked(&self, lifecycle: &mut LifecycleAuthority) {
        if lifecycle.latch_commit_indeterminate() {
            self.publish_storage_unavailable();
        }
    }

    fn enqueue_live(
        &self,
        lifecycle: &mut LifecycleAuthority,
        occurrence: HookEnvelope,
        producer: &CodexLiveProducerProof,
    ) -> Result<CommittedLiveEvent, CodexHookRejection> {
        let replay_identity = VerifiedLiveOccurrence::optional(self, &occurrence)
            .map_err(CodexHookRejection::Storage)?
            .map(|occurrence| occurrence.durable_identity())
            .transpose()
            .map_err(CodexHookRejection::Storage)?;
        let mut commit_fence = None;
        let result = self.outbox.enqueue_admitted_decision(
            occurrence,
            replay_identity.as_ref(),
            |assigned, admission| {
                let occurrence = producer
                    .authenticate(self, assigned, admission)
                    .map_err(anyhow::Error::new)?;
                let mut candidate = lifecycle.reducer()?.clone();
                let installed_revision = candidate.revision();
                match candidate.apply(assigned.clone()) {
                    Ok(disposition) => {
                        ensure!(
                            disposition != ApplyDisposition::Invalidated,
                            super::super::constants::CODEX_REDUCER_ERROR
                        );
                        Ok(crate::codex_hooks::CodexEnqueueDecision::accepted(
                            occurrence,
                        ))
                    }
                    Err(violation)
                        if matches!(admission, crate::codex_hooks::CodexEnqueueAdmission::New)
                            && violation.is_durable_rejection()
                            && candidate.revision() > installed_revision
                            && candidate.is_revoked() =>
                    {
                        Ok(crate::codex_hooks::CodexEnqueueDecision::rejected(
                            occurrence, violation,
                        ))
                    }
                    Err(violation)
                        if matches!(
                            admission,
                            crate::codex_hooks::CodexEnqueueAdmission::Replay
                        ) =>
                    {
                        Ok(crate::codex_hooks::CodexEnqueueDecision::accepted(
                            occurrence,
                        ))
                    }
                    Err(violation) => Err(anyhow::Error::new(violation)),
                }
            },
            |assigned, admission, occurrence| {
                producer
                    .verify_commit(self, assigned, admission, occurrence.as_ref())
                    .map_err(anyhow::Error::new)?;
                commit_fence = Some(
                    producer
                        .acquire_generation_fence()
                        .map_err(anyhow::Error::new)?,
                );
                Ok(())
            },
        );
        let result = result.map_err(classify_enqueue_error)?;
        let generation_fence = match commit_fence {
            Some(fence) => fence,
            None => producer.acquire_generation_fence()?,
        };
        let (event, occurrence, durable_rejection) = result.into_parts();
        Ok(CommittedLiveEvent {
            event,
            occurrence,
            durable_rejection,
            generation_fence,
        })
    }
}

fn classify_enqueue_error(error: anyhow::Error) -> CodexHookRejection {
    if crate::codex_hooks::is_outbox_commit_indeterminate(&error) {
        return CodexHookRejection::CommitIndeterminate(error);
    }
    let error = match error.downcast::<CodexHookRejection>() {
        Ok(rejection) => return rejection,
        Err(error) => error,
    };
    match error.downcast::<HookViolation>() {
        Ok(violation) => CodexHookRejection::Violation(violation),
        Err(error) => CodexHookRejection::Storage(error),
    }
}
