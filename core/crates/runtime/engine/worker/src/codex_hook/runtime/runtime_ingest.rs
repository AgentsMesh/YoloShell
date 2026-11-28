use super::super::authority::{CommittedReduction, LifecycleAuthority};
use super::super::constants::{
    CODEX_DURABLE_EVENT_ERROR, CODEX_OWNER_ERROR, CODEX_STORAGE_LATCHED_ERROR,
};
use super::super::error::CodexHookRejection;
use super::super::outbox_authority::RejectionExpectation;
use super::{CodexHookPublication, CodexHookRuntime, CodexIngressEvidence, CommittedLiveEvent};
use crate::agent::hooks::{HookEnvelope, HookViolation};

pub(super) struct ProvenCodexPublication {
    pub(super) publication: CodexHookPublication,
    pub(super) rejection: Option<HookViolation>,
}

impl CodexHookRuntime {
    pub(super) fn ingest_locked(
        &self,
        lifecycle: &mut LifecycleAuthority,
        event: HookEnvelope,
        evidence: &CodexIngressEvidence<'_>,
        occurrence_identity: Option<&crate::codex_hooks::CodexOccurrenceIdentity>,
    ) -> Result<ProvenCodexPublication, CodexHookRejection> {
        self.prepare_ingest(lifecycle, &event)?;
        self.admit_event(&event, evidence)?;
        self.commit_ingest(
            lifecycle,
            event,
            occurrence_identity,
            RejectionExpectation::AuthoritativeOutbox,
            &mut || Ok(()),
        )
    }

    pub(super) fn ingest_committed_locked(
        &self,
        lifecycle: &mut LifecycleAuthority,
        committed: &CommittedLiveEvent,
        mut publication_fence: impl FnMut() -> Result<(), CodexHookRejection>,
    ) -> Result<ProvenCodexPublication, CodexHookRejection> {
        self.prepare_ingest(lifecycle, &committed.event)?;
        let identity = committed
            .occurrence
            .as_ref()
            .map(super::VerifiedLiveOccurrence::durable_identity)
            .transpose()
            .map_err(CodexHookRejection::Storage)?;
        self.commit_ingest(
            lifecycle,
            committed.event.clone(),
            identity.as_ref(),
            RejectionExpectation::Exact(committed.durable_rejection),
            &mut publication_fence,
        )
    }

    fn prepare_ingest(
        &self,
        lifecycle: &mut LifecycleAuthority,
        event: &HookEnvelope,
    ) -> Result<(), CodexHookRejection> {
        #[cfg(test)]
        self.panic_ingest_for_test();
        if lifecycle.storage_failed() {
            return Err(CodexHookRejection::Storage(anyhow::anyhow!(
                CODEX_STORAGE_LATCHED_ERROR
            )));
        }
        let proof = match lifecycle.prove_snapshot_current() {
            Ok(proof) => proof,
            Err(error) => return Err(self.reject_authority_error(lifecycle, error, None)),
        };
        if let Err(error) = lifecycle.ensure_operable() {
            return Err(self.reject_authority_error(lifecycle, error, Some(&proof)));
        }
        if lifecycle.owner() != &event.identity {
            return Err(CodexHookRejection::Identity(CODEX_OWNER_ERROR));
        }
        Ok(())
    }

    fn commit_ingest(
        &self,
        lifecycle: &mut LifecycleAuthority,
        event: HookEnvelope,
        occurrence_identity: Option<&crate::codex_hooks::CodexOccurrenceIdentity>,
        rejection_expectation: RejectionExpectation,
        publication_fence: &mut impl FnMut() -> Result<(), CodexHookRejection>,
    ) -> Result<ProvenCodexPublication, CodexHookRejection> {
        publication_fence()?;
        let result = super::super::outbox_authority::transact(
            &self.outbox,
            lifecycle,
            event,
            occurrence_identity,
            rejection_expectation,
        );
        let proof = match lifecycle.prove_snapshot_current() {
            Ok(proof) => proof,
            Err(error) => return Err(self.reject_authority_error(lifecycle, error, None)),
        };
        match result {
            Ok(super::super::outbox_authority::DurableEvent::Proven(
                CommittedReduction::Accepted(publication),
            )) => {
                publication_fence()?;
                self.publish_accepted(lifecycle, publication, proof)
            }
            Ok(super::super::outbox_authority::DurableEvent::Proven(
                CommittedReduction::Rejected(violation, state),
            )) => {
                publication_fence()?;
                if let Err(error) = self.publish_lifecycle(&proof, lifecycle) {
                    return Err(self.reject_authority_error(lifecycle, error, None));
                }
                Ok(ProvenCodexPublication {
                    publication: CodexHookPublication {
                        disposition: crate::agent::hooks::ApplyDisposition::Invalidated,
                        state,
                    },
                    rejection: Some(violation),
                })
            }
            Ok(super::super::outbox_authority::DurableEvent::RetainedReplay) => {
                publication_fence()?;
                let state = lifecycle.lifecycle().ok_or(CodexHookRejection::Internal)?;
                self.publish_accepted(
                    lifecycle,
                    CodexHookPublication {
                        disposition: crate::agent::hooks::ApplyDisposition::Replay,
                        state,
                    },
                    proof,
                )
            }
            Ok(super::super::outbox_authority::DurableEvent::Missing) => {
                Err(CodexHookRejection::Outbox(CODEX_DURABLE_EVENT_ERROR))
            }
            Err(error) => Err(self.reject_authority_error(lifecycle, error, Some(&proof))),
        }
    }

    fn publish_accepted(
        &self,
        lifecycle: &mut LifecycleAuthority,
        publication: CodexHookPublication,
        proof: crate::codex_hooks::CodexSnapshotProof,
    ) -> Result<ProvenCodexPublication, CodexHookRejection> {
        if let Err(error) = self.publish_lifecycle(&proof, lifecycle) {
            return Err(self.reject_authority_error(lifecycle, error, None));
        }
        Ok(ProvenCodexPublication {
            publication,
            rejection: None,
        })
    }
}
