use super::authority::{CommittedReduction, LifecycleAuthority};
use super::constants::CODEX_OUTBOX_HISTORY_ERROR;
use crate::agent::hooks::{
    AUTHORITY_REVISION_STEP, ApplyDisposition, HookEnvelope, HookViolation, NormalizedLifecycle,
    Reducer,
};
use crate::codex_hooks::CodexDurableRejection;
use anyhow::{Context, Result, ensure};

enum ProbeOutcome {
    Accepted(ApplyDisposition),
    Rejected(HookViolation),
}

struct ReductionProbe {
    outcome: ProbeOutcome,
    state: NormalizedLifecycle,
    revision: u64,
}

pub(super) fn commit(
    authority: &mut LifecycleAuthority,
    event: HookEnvelope,
    marker: Option<CodexDurableRejection>,
) -> Result<CommittedReduction> {
    let probe = ReductionProbe::evaluate(authority.reducer()?, &event, marker)?;
    let committed = authority.apply_committed(event)?;
    probe.verify(authority.reducer()?, &committed)?;
    Ok(committed)
}

impl ReductionProbe {
    fn evaluate(
        installed: &Reducer,
        event: &HookEnvelope,
        marker: Option<CodexDurableRejection>,
    ) -> Result<Self> {
        if let Some(marker) = marker {
            marker
                .validate_for(event)
                .context(CODEX_OUTBOX_HISTORY_ERROR)?;
        }
        let expected_revision = installed
            .revision()
            .checked_add(AUTHORITY_REVISION_STEP)
            .context(CODEX_OUTBOX_HISTORY_ERROR)?;
        let mut candidate = installed.clone();
        let result = candidate.apply(event.clone());
        ensure!(
            candidate.revision() == expected_revision,
            CODEX_OUTBOX_HISTORY_ERROR
        );
        let outcome = match (marker, result) {
            (None, Ok(disposition)) => ProbeOutcome::Accepted(disposition),
            (Some(expected), Err(actual)) => {
                ensure!(expected.violation() == actual, CODEX_OUTBOX_HISTORY_ERROR);
                ensure!(actual.is_durable_rejection(), CODEX_OUTBOX_HISTORY_ERROR);
                ensure!(candidate.state().revoked, CODEX_OUTBOX_HISTORY_ERROR);
                let (rejected, proof) = candidate
                    .revocation_evidence()
                    .and_then(|evidence| evidence.hook_rejection())
                    .context(CODEX_OUTBOX_HISTORY_ERROR)?;
                ensure!(rejected == event, CODEX_OUTBOX_HISTORY_ERROR);
                ensure!(proof == expected.proof(), CODEX_OUTBOX_HISTORY_ERROR);
                ProbeOutcome::Rejected(actual)
            }
            (None, Err(actual)) if !actual.is_durable_rejection() => {
                return Err(anyhow::Error::new(actual).context(CODEX_OUTBOX_HISTORY_ERROR));
            }
            _ => return Err(anyhow::anyhow!(CODEX_OUTBOX_HISTORY_ERROR)),
        };
        Ok(Self {
            outcome,
            state: candidate.state().clone(),
            revision: candidate.revision(),
        })
    }

    fn verify(&self, committed: &Reducer, reduction: &CommittedReduction) -> Result<()> {
        ensure!(
            committed.revision() == self.revision,
            CODEX_OUTBOX_HISTORY_ERROR
        );
        ensure!(committed.state() == &self.state, CODEX_OUTBOX_HISTORY_ERROR);
        match (&self.outcome, reduction) {
            (ProbeOutcome::Accepted(expected), CommittedReduction::Accepted(actual)) => {
                ensure!(actual.disposition == *expected, CODEX_OUTBOX_HISTORY_ERROR);
                ensure!(actual.state == self.state, CODEX_OUTBOX_HISTORY_ERROR);
            }
            (ProbeOutcome::Rejected(expected), CommittedReduction::Rejected(actual, state)) => {
                ensure!(actual == expected, CODEX_OUTBOX_HISTORY_ERROR);
                ensure!(state == &self.state, CODEX_OUTBOX_HISTORY_ERROR);
                ensure!(state.revoked, CODEX_OUTBOX_HISTORY_ERROR);
            }
            _ => return Err(anyhow::anyhow!(CODEX_OUTBOX_HISTORY_ERROR)),
        }
        Ok(())
    }
}
