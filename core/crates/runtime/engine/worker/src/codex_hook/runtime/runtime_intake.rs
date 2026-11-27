use super::super::router::CodexSessionRouter;
#[cfg(test)]
use super::super::{constants::CODEX_OWNER_ERROR, error::CodexHookRejection};
use super::CodexHookRuntime;
use super::CodexIngressEvidence;
use super::intake_admission::VerifiedDurableOccurrence;
use super::intake_consume::OccurrenceOutcome;
use crate::agent::hooks::ApplyDisposition;
#[cfg(test)]
use crate::agent::hooks::{EventSequence, HookEnvelope};
use crate::codex_hooks::CodexHookIntake;
#[cfg(test)]
use crate::codex_hooks::CodexOccurrenceIdentity;
use anyhow::Result;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::worker::codex_hook) struct IntakeReplayReport {
    pub(in crate::worker::codex_hook) applied: usize,
    pub(in crate::worker::codex_hook) replayed: usize,
    pub(in crate::worker::codex_hook) stale: usize,
    pub(in crate::worker::codex_hook) quarantined: usize,
    pub(in crate::worker::codex_hook) deferred: usize,
}

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn scan_unrouted_intake(&self) -> Result<IntakeReplayReport> {
        let intake = CodexHookIntake::for_identity(self.artifacts.intake_directory(), &self.owner)?;
        intake.recover_live_receipts(&self.outbox)?;
        let scan = intake.scan()?;
        Ok(IntakeReplayReport {
            quarantined: scan.quarantined,
            deferred: scan.deferred.saturating_add(scan.occurrences.len()),
            ..IntakeReplayReport::default()
        })
    }

    pub(in crate::worker::codex_hook) fn reconcile_routed_intake(
        &self,
        router: &CodexSessionRouter,
    ) -> Result<IntakeReplayReport> {
        let intake = CodexHookIntake::for_identity(self.artifacts.intake_directory(), &self.owner)?;
        intake.recover_live_receipts(&self.outbox)?;
        let scan = intake.scan()?;
        let mut report = IntakeReplayReport {
            quarantined: scan.quarantined,
            deferred: scan.deferred,
            ..IntakeReplayReport::default()
        };
        for occurrence in scan.occurrences {
            let evidence =
                match VerifiedDurableOccurrence::authorize(self, router, &intake, &occurrence) {
                    Ok(proof) => CodexIngressEvidence::VerifiedDurableOccurrence(proof),
                    Err(_) => {
                        report.deferred = report.deferred.saturating_add(1);
                        break;
                    }
                };
            match self.consume_authorized(&intake, &occurrence, evidence) {
                Ok(OccurrenceOutcome::Applied(disposition)) => {
                    report.record(disposition);
                }
                Ok(OccurrenceOutcome::Rejected(_)) => {}
                Ok(OccurrenceOutcome::Poison) => {
                    report.quarantined = report.quarantined.saturating_add(1);
                }
                Ok(OccurrenceOutcome::Unproven) => {
                    report.deferred = report.deferred.saturating_add(1);
                }
                Err(_) => {
                    report.deferred = report.deferred.saturating_add(1);
                    break;
                }
            }
        }
        Ok(report)
    }

    #[cfg(test)]
    pub(super) fn test_occurrence_identity(
        &self,
        event: &HookEnvelope,
    ) -> Result<Option<CodexOccurrenceIdentity>, CodexHookRejection> {
        self.validate_event_scope(event)?;
        if event.identity != self.owner {
            return Err(CodexHookRejection::Identity(CODEX_OWNER_ERROR));
        }
        if event.event_id.is_nil() || event.sequence == EventSequence::ZERO {
            return Ok(None);
        }
        let intake = CodexHookIntake::for_identity(self.artifacts.intake_directory(), &self.owner)
            .map_err(CodexHookRejection::Storage)?;
        let mut occurrence = event.clone();
        occurrence.sequence = EventSequence::ZERO;
        let Some(occurrence) = intake
            .find_exact_occurrence(&occurrence)
            .map_err(CodexHookRejection::Storage)?
        else {
            return Ok(None);
        };
        occurrence
            .durable_identity()
            .map(Some)
            .map_err(CodexHookRejection::Storage)
    }
}

impl IntakeReplayReport {
    pub(super) fn record(&mut self, disposition: ApplyDisposition) {
        let counter = match disposition {
            ApplyDisposition::Applied => &mut self.applied,
            ApplyDisposition::Replay => &mut self.replayed,
            ApplyDisposition::Stale => &mut self.stale,
            ApplyDisposition::Invalidated => &mut self.deferred,
        };
        *counter = counter.saturating_add(1);
    }
}
