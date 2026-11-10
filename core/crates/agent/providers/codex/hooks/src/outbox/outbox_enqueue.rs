use super::constants::{FIRST_SEQUENCE, MAX_PENDING_EVENTS, errors};
use super::lock::OutboxLock;
use super::outbox::CodexOutbox;
use super::{outbox_storage, outbox_validation};
use crate::agent::hooks::{EventSequence, HookEnvelope, HookViolation, ZERO_SEQUENCE};
use anyhow::{Context, Result, ensure};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexEnqueueAdmission {
    New,
    Replay,
}

pub struct CodexEnqueueResult<T> {
    event: HookEnvelope,
    admission: T,
    durable_rejection: Option<super::CodexDurableRejection>,
}

pub struct CodexEnqueueDecision<T> {
    value: T,
    durable_rejection: Option<HookViolation>,
}

impl<T> CodexEnqueueResult<T> {
    pub fn into_parts(self) -> (HookEnvelope, T, Option<super::CodexDurableRejection>) {
        (self.event, self.admission, self.durable_rejection)
    }
}

impl<T> CodexEnqueueDecision<T> {
    pub fn accepted(value: T) -> Self {
        Self {
            value,
            durable_rejection: None,
        }
    }

    pub fn rejected(value: T, violation: HookViolation) -> Self {
        Self {
            value,
            durable_rejection: Some(violation),
        }
    }
}

impl CodexOutbox {
    pub fn enqueue(&self, envelope: HookEnvelope) -> Result<HookEnvelope> {
        self.enqueue_admitted(envelope, None, |_, _| Ok(()), |_, _, _| Ok(()))
            .map(|result| result.event)
    }

    pub(crate) fn enqueue_admitted<T>(
        &self,
        envelope: HookEnvelope,
        occurrence_identity: Option<&super::CodexOccurrenceIdentity>,
        admit: impl FnOnce(&HookEnvelope, CodexEnqueueAdmission) -> Result<T>,
        commit_fence: impl FnMut(&HookEnvelope, CodexEnqueueAdmission, &T) -> Result<()>,
    ) -> Result<CodexEnqueueResult<T>> {
        self.enqueue_admitted_decision(
            envelope,
            occurrence_identity,
            |event, admission| admit(event, admission).map(CodexEnqueueDecision::accepted),
            commit_fence,
        )
    }

    pub fn enqueue_admitted_decision<T>(
        &self,
        mut envelope: HookEnvelope,
        occurrence_identity: Option<&super::CodexOccurrenceIdentity>,
        admit: impl FnOnce(&HookEnvelope, CodexEnqueueAdmission) -> Result<CodexEnqueueDecision<T>>,
        mut commit_fence: impl FnMut(&HookEnvelope, CodexEnqueueAdmission, &T) -> Result<()>,
    ) -> Result<CodexEnqueueResult<T>> {
        let lock = OutboxLock::acquire(&self.directory, &self.path)?;
        let mut loaded = outbox_storage::load(&lock, &self.path)?;
        outbox_validation::bind(&mut loaded.state, self.owner.as_ref(), &envelope.identity)?;
        let unassigned = envelope.event_id.is_nil();
        if unassigned {
            ensure!(
                envelope.sequence.0 == ZERO_SEQUENCE && occurrence_identity.is_none(),
                errors::OUTBOX_CORRUPT
            );
        } else if let Some(existing) =
            outbox_validation::find_explicit_replay(&loaded.state, &envelope, occurrence_identity)?
        {
            let decision = admit(&existing, CodexEnqueueAdmission::Replay)?;
            let durable_rejection = loaded.state.durable_rejection_for(&existing)?;
            ensure!(
                decision.durable_rejection.is_none()
                    || durable_rejection.is_some_and(
                        |marker| Some(marker.violation()) == decision.durable_rejection
                    ),
                errors::OUTBOX_CORRUPT
            );
            return Ok(CodexEnqueueResult {
                event: existing,
                admission: decision.value,
                durable_rejection,
            });
        }
        let occurrence = occurrence_identity.map(|_| envelope.clone());
        let expected = loaded.state.next_sequence;
        if envelope.sequence.0 != ZERO_SEQUENCE {
            ensure!(envelope.sequence.0 == expected, errors::OUTBOX_CORRUPT);
        }
        ensure!(
            loaded.state.pending.len() < MAX_PENDING_EVENTS,
            errors::OUTBOX_FULL
        );
        envelope.sequence = EventSequence(expected);
        if unassigned {
            envelope.event_id = outbox_validation::derive_allocated_event_id(&envelope)?;
            ensure!(
                outbox_validation::find_explicit_replay(&loaded.state, &envelope, None)?.is_none(),
                errors::OUTBOX_CORRUPT
            );
        }
        let witness = occurrence_identity
            .zip(occurrence.as_ref())
            .map(|(identity, occurrence)| {
                super::outbox_pending_witness::CodexPendingOccurrenceWitness::capture(
                    &envelope, occurrence, identity,
                )
            })
            .transpose()?;
        if witness.is_some() {
            super::outbox_occurrence_state::ensure_capacity(&loaded.state)?;
        }
        let decision = admit(&envelope, CodexEnqueueAdmission::New)?;
        let durable_rejection = decision
            .durable_rejection
            .map(|violation| {
                super::outbox_rejection::insert(&mut loaded.state, &envelope, violation)
            })
            .transpose()?;
        loaded.state.next_sequence = expected
            .checked_add(FIRST_SEQUENCE)
            .context(errors::OUTBOX_CORRUPT)?;
        loaded.state.pending.push_back(envelope.clone());
        if let Some(witness) = witness {
            super::outbox_pending_witness_state::insert(&mut loaded.state, witness)?;
        }
        outbox_storage::persist_with_commit_fence(&lock, &self.path, &loaded, || {
            commit_fence(&envelope, CodexEnqueueAdmission::New, &decision.value)
        })?;
        Ok(CodexEnqueueResult {
            event: envelope,
            admission: decision.value,
            durable_rejection,
        })
    }
}
