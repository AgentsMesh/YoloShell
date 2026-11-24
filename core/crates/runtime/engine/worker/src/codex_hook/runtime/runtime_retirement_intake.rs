use super::intake::IntakeReplayReport;
use super::intake_consume::OccurrenceOutcome;
use super::{CodexHookRuntime, CodexIngressEvidence, CodexRetirementIntakeAuthority};
use crate::codex_hooks::CodexHookIntake;
use anyhow::Result;

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn reconcile_retirement_intake(
        &self,
        authority: &CodexRetirementIntakeAuthority<'_>,
        intake: &CodexHookIntake,
    ) -> Result<IntakeReplayReport> {
        authority.recover_receipts(self, intake)?;
        let scan = intake.scan()?;
        let mut report = IntakeReplayReport {
            quarantined: scan.quarantined,
            deferred: scan.deferred,
            ..IntakeReplayReport::default()
        };
        for occurrence in scan.occurrences {
            let evidence = match authority.authorize(self, intake, &occurrence) {
                Ok(proof) => CodexIngressEvidence::VerifiedRetirementOccurrence(proof),
                Err(_) => {
                    report.deferred = report.deferred.saturating_add(1);
                    break;
                }
            };
            match self.consume_authorized(intake, &occurrence, evidence) {
                Ok(OccurrenceOutcome::Applied(disposition)) => report.record(disposition),
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
}
