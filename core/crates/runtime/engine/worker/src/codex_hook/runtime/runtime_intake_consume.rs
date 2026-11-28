use super::super::error::CodexHookRejection;
use super::{CodexHookRuntime, CodexIngressEvidence};
use crate::agent::hooks::{ApplyDisposition, HookEnvelope, HookViolation};
use crate::codex_hooks::{CodexHookIntake, CodexIntakeOccurrence};
use anyhow::{Context, Result};

pub(super) enum OccurrenceOutcome {
    Applied(ApplyDisposition),
    Rejected(HookViolation),
    Poison,
    Unproven,
}

enum Finalization {
    Complete,
    Quarantine,
}

impl CodexHookRuntime {
    pub(super) fn consume_authorized(
        &self,
        intake: &CodexHookIntake,
        occurrence: &CodexIntakeOccurrence,
        ingress: CodexIngressEvidence<'_>,
    ) -> Result<OccurrenceOutcome> {
        #[cfg(test)]
        self.wait_for_intake_authorization_gate_for_test();
        let mut lifecycle = self.lock_lifecycle_anyhow()?;
        #[cfg(test)]
        self.wait_for_intake_fold_gate_for_test();
        self.verify_intake_evidence(intake, occurrence.envelope(), &ingress)?;
        let event = match self.outbox.authoritative_intake_occurrence(occurrence)? {
            Some(event) => event,
            None if matches!(
                ingress,
                CodexIngressEvidence::VerifiedRetirementOccurrence(_)
            ) =>
            {
                self.finalize_occurrence(intake, occurrence, &ingress, Finalization::Quarantine)?;
                return Ok(OccurrenceOutcome::Poison);
            }
            None => return Ok(OccurrenceOutcome::Unproven),
        };
        let occurrence_identity = occurrence.durable_identity()?;
        let committed = self
            .ingest_locked(
                &mut lifecycle,
                event.clone(),
                &ingress,
                Some(&occurrence_identity),
            )
            .map_err(rejection_error)?;
        let acknowledgement = self
            .outbox
            .acknowledge_occurrence(&event, occurrence.envelope(), &occurrence_identity)
            .context(super::super::constants::CODEX_STORAGE_ERROR);
        if let Err(error) = acknowledgement {
            if crate::codex_hooks::is_outbox_commit_indeterminate(&error) {
                self.latch_commit_indeterminate_locked(&mut lifecycle);
            }
            return Err(error);
        }
        self.finalize_occurrence(intake, occurrence, &ingress, Finalization::Complete)?;
        let receipt_removal = self
            .outbox
            .remove_occurrence_receipt(&event, occurrence.envelope(), &occurrence_identity)
            .context(super::super::constants::CODEX_STORAGE_ERROR);
        if let Err(error) = receipt_removal {
            if crate::codex_hooks::is_outbox_commit_indeterminate(&error) {
                self.latch_commit_indeterminate_locked(&mut lifecycle);
            }
            return Err(error);
        }
        Ok(match committed.rejection {
            Some(violation) => OccurrenceOutcome::Rejected(violation),
            None => OccurrenceOutcome::Applied(committed.publication.disposition),
        })
    }

    fn finalize_occurrence(
        &self,
        intake: &CodexHookIntake,
        occurrence: &CodexIntakeOccurrence,
        ingress: &CodexIngressEvidence<'_>,
        finalization: Finalization,
    ) -> Result<()> {
        self.verify_intake_evidence(intake, occurrence.envelope(), ingress)?;
        match finalization {
            Finalization::Complete => intake.complete(occurrence)?,
            Finalization::Quarantine => intake.quarantine(occurrence)?,
        }
        self.verify_intake_evidence(intake, occurrence.envelope(), ingress)
    }

    fn verify_intake_evidence(
        &self,
        intake: &CodexHookIntake,
        event: &HookEnvelope,
        ingress: &CodexIngressEvidence<'_>,
    ) -> Result<()> {
        match ingress {
            CodexIngressEvidence::VerifiedDurableOccurrence(proof) => {
                proof.verify_intake(self, intake, event)
            }
            CodexIngressEvidence::VerifiedRetirementOccurrence(proof) => {
                proof.verify_intake(self, intake, event)
            }
            #[cfg(test)]
            CodexIngressEvidence::TestPeer(_) => Err(CodexHookRejection::Identity(
                super::super::constants::CODEX_DURABLE_EVENT_ERROR,
            )),
        }
        .map_err(rejection_error)
    }
}

fn rejection_error(rejection: CodexHookRejection) -> anyhow::Error {
    anyhow::Error::new(rejection)
}
