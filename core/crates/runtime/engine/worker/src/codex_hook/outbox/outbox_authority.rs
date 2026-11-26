use super::authority::{CommittedReduction, LifecycleAuthority};
use super::constants::{
    CODEX_OUTBOX_HISTORY_ERROR, CODEX_OUTBOX_TIP_ERROR, CODEX_SNAPSHOT_WATERMARK_ERROR,
};
use super::marker_tail;
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::{CodexDurableRejection, CodexOutbox, CodexOutboxState};
use anyhow::{Context, Result, ensure};

pub(super) enum DurableEvent<T> {
    Proven(T),
    RetainedReplay,
    Missing,
}

#[derive(Clone, Copy)]
pub(super) enum RejectionExpectation {
    AuthoritativeOutbox,
    Exact(Option<CodexDurableRejection>),
}

enum EventProof {
    PendingHead(Option<CodexDurableRejection>),
    RejectedReplay(CodexDurableRejection),
    RetainedReplay,
    Missing,
}

pub(super) fn transact(
    outbox: &CodexOutbox,
    authority: &mut LifecycleAuthority,
    incoming: HookEnvelope,
    occurrence_identity: Option<&crate::codex_hooks::CodexOccurrenceIdentity>,
    rejection_expectation: RejectionExpectation,
) -> Result<DurableEvent<CommittedReduction>> {
    with_locked_state(outbox, |state| {
        rejection_expectation.validate(state.durable_rejection_for(&incoming)?)?;
        match prove_event(state, authority, &incoming, occurrence_identity)? {
            EventProof::PendingHead(expected_rejection) => {
                let reduction = super::reduction_probe::commit(
                    authority,
                    incoming.clone(),
                    expected_rejection,
                )?;
                Ok(DurableEvent::Proven(reduction))
            }
            EventProof::RejectedReplay(marker) => {
                let state = authority.reducer()?.state().clone();
                Ok(DurableEvent::Proven(CommittedReduction::Rejected(
                    marker.violation(),
                    state,
                )))
            }
            EventProof::RetainedReplay => Ok(DurableEvent::RetainedReplay),
            EventProof::Missing => Ok(DurableEvent::Missing),
        }
    })
}

impl RejectionExpectation {
    fn validate(self, actual: Option<CodexDurableRejection>) -> Result<()> {
        if let Self::Exact(expected) = self {
            ensure!(expected == actual, CODEX_OUTBOX_HISTORY_ERROR);
        }
        Ok(())
    }
}

pub(super) fn with_locked_state<T>(
    outbox: &CodexOutbox,
    operation: impl FnOnce(&CodexOutboxState) -> Result<T>,
) -> Result<T> {
    outbox.with_locked_state(operation)
}

fn prove_event(
    state: &CodexOutboxState,
    authority: &LifecycleAuthority,
    incoming: &HookEnvelope,
    occurrence_identity: Option<&crate::codex_hooks::CodexOccurrenceIdentity>,
) -> Result<EventProof> {
    let installed = validate_live_authority(state, authority)?;
    let retained = marker_tail::exact_retained(state, incoming)?;
    let durable_rejection = state.durable_rejection_for(incoming)?;
    if occurrence_identity.is_none() && state.has_occurrence_binding(incoming) {
        return Ok(EventProof::Missing);
    }
    let receipt_proven = occurrence_identity
        .map(|identity| state.proves_occurrence_receipt(incoming, identity))
        .transpose()?
        .unwrap_or(false);
    if let Some(marker) = durable_rejection {
        let Some(acknowledged) = retained else {
            return Ok(EventProof::Missing);
        };
        if receipt_proven {
            ensure!(acknowledged, CODEX_OUTBOX_HISTORY_ERROR);
            return Ok(EventProof::RejectedReplay(marker));
        }
        let witness_proven = occurrence_identity
            .map(|identity| state.proves_pending_occurrence_witness(incoming, identity))
            .transpose()?
            .unwrap_or(false);
        if let Some(head) = state.pending_head() {
            if head == incoming {
                if occurrence_identity.is_some() && !witness_proven {
                    return Ok(EventProof::Missing);
                }
                return Ok(if acknowledged || authority.reducer()?.is_revoked() {
                    EventProof::RejectedReplay(marker)
                } else {
                    EventProof::PendingHead(Some(marker))
                });
            }
        }
        if acknowledged && occurrence_identity.is_none() {
            return Ok(EventProof::RejectedReplay(marker));
        }
        return Ok(EventProof::Missing);
    }
    if receipt_proven {
        return Ok(EventProof::RetainedReplay);
    }
    let witness_proven = occurrence_identity
        .map(|identity| state.proves_pending_occurrence_witness(incoming, identity))
        .transpose()?
        .unwrap_or(false);
    if let Some(acknowledged) = retained {
        if acknowledged {
            if occurrence_identity.is_some() {
                return Ok(EventProof::Missing);
            }
            return Ok(EventProof::RetainedReplay);
        }
        if occurrence_identity.is_some() && !witness_proven {
            return Ok(EventProof::Missing);
        }
    }
    if state.pending_head() == Some(incoming) {
        Ok(if incoming.sequence.0 <= installed {
            EventProof::RetainedReplay
        } else {
            EventProof::PendingHead(None)
        })
    } else {
        Ok(EventProof::Missing)
    }
}

fn validate_live_authority(
    state: &CodexOutboxState,
    authority: &LifecycleAuthority,
) -> Result<u64> {
    let tip = state
        .next_sequence()
        .checked_sub(crate::agent::hooks::FIRST_SEQUENCE)
        .context(CODEX_OUTBOX_TIP_ERROR)?;
    let reducer = authority.reducer()?;
    let installed = reducer.state().last_sequence.0;
    ensure!(installed <= tip, CODEX_OUTBOX_TIP_ERROR);
    crate::codex_hooks::validate_outbox_watermark(state, installed)
        .context(CODEX_SNAPSHOT_WATERMARK_ERROR)?;
    marker_tail::validate_owner(
        state,
        authority.owner(),
        tip != crate::agent::hooks::ZERO_SEQUENCE,
    )?;
    marker_tail::validate(state, reducer, installed, tip)?;
    marker_tail::validate_overlap(state, reducer, installed)?;
    Ok(installed)
}
