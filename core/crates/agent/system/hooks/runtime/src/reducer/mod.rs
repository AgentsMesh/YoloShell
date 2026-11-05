pub(crate) mod canonical;
pub(crate) mod degraded;
pub(crate) mod history;
pub(crate) mod revocation;

use crate::admission::{is_unknown_child_gap, validate_envelope, validate_topology};
use crate::event::HookEnvelope;
use crate::identity::{
    EventSequence, ExpectedIdentity, ProcessIdentity, HookProviderKind, ProviderVersion,
};
use crate::reducer::canonical::normalize_inactive_state;
use crate::snapshot::revision::AuthorityRevision;
use crate::state::{InstrumentationState, NormalizedLifecycle, ProcessState, TurnState};
use crate::transition::admission::{EventAdmission, admit_active_event, admit_terminal_event};
use crate::transition::apply_event;
use crate::violation::HookViolation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplyDisposition {
    Applied,
    Replay,
    Stale,
    Invalidated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessExitDisposition {
    Confirmed,
    Unconfirmed,
    AlreadyExited,
    Revoked,
    AuthorityExhausted,
}

#[derive(Clone, Debug)]
pub struct Reducer {
    pub(crate) state: NormalizedLifecycle,
    pub(crate) revision: AuthorityRevision,
}

impl Reducer {
    pub fn new(
        expected_identity: ExpectedIdentity,
        provider: HookProviderKind,
        provider_version: ProviderVersion,
    ) -> Result<Self, HookViolation> {
        expected_identity.validate()?;
        Ok(Self {
            state: NormalizedLifecycle::new(expected_identity, provider, provider_version),
            revision: AuthorityRevision::initial(),
        })
    }

    pub fn state(&self) -> &NormalizedLifecycle {
        &self.state
    }

    pub fn revision(&self) -> u64 {
        self.revision.get()
    }

    pub fn is_revoked(&self) -> bool {
        self.state.revoked
    }

    pub fn authority_exhausted(&self) -> bool {
        !self.state.revoked
            && self.state.process == ProcessState::Alive
            && self.revision.is_exhausted()
    }

    pub fn apply(&mut self, envelope: HookEnvelope) -> Result<ApplyDisposition, HookViolation> {
        if self.state.revoked {
            return Err(HookViolation::Revoked);
        }
        if let Err(violation) = validate_envelope(&self.state, &envelope) {
            return self.invalidate_event(&envelope, violation);
        }
        if let Some(previous) = self.state.history.get(&envelope.sequence) {
            return if previous == &envelope {
                Ok(ApplyDisposition::Replay)
            } else {
                self.invalidate_event(&envelope, HookViolation::DuplicateConflict)
            };
        }
        if self.state.process == ProcessState::Exited {
            return Err(HookViolation::ProcessAlreadyExited);
        }
        if self.state.event_ids.contains_key(&envelope.event_id) {
            return self.invalidate_event(&envelope, HookViolation::DuplicateConflict);
        }
        let expected = self
            .state
            .last_sequence
            .next()
            .unwrap_or(EventSequence::ZERO);
        if envelope.sequence != expected {
            return self.invalidate_event(&envelope, HookViolation::SequenceGap);
        }
        if crate::reducer::degraded::is_active(&self.state)
            && !crate::reducer::degraded::restores_authority(&envelope)
        {
            return self.record_degraded(envelope, ApplyDisposition::Stale);
        }
        let terminal = match admit_terminal_event(&self.state, &envelope) {
            Ok(admission) => admission,
            Err(violation) => return self.invalidate_event(&envelope, violation),
        };
        let admission = if let Some(admission) = terminal {
            admission
        } else {
            if let Err(violation) = validate_topology(&self.state, &envelope) {
                return if is_unknown_child_gap(&self.state, &envelope) {
                    self.record_degraded(envelope, ApplyDisposition::Applied)
                } else {
                    self.invalidate_event(&envelope, violation)
                };
            }
            admit_active_event(&self.state, &envelope)
        };
        if admission == EventAdmission::Apply
            && !crate::state::capacity::admits_transition(&self.state, &envelope)
        {
            return self.record_degraded(envelope, ApplyDisposition::Applied);
        }
        self.revision.advance()?;
        if admission == EventAdmission::Apply {
            apply_event(&mut self.state, &envelope);
        }
        crate::lifecycle_metadata::apply(&mut self.state, &envelope);
        crate::reducer::history::record(&mut self.state, envelope);
        Ok(match admission {
            EventAdmission::Apply => ApplyDisposition::Applied,
            EventAdmission::Stale => ApplyDisposition::Stale,
        })
    }

    pub fn process_exit(&mut self) -> ProcessExitDisposition {
        if self.state.revoked {
            return ProcessExitDisposition::Revoked;
        }
        if self.state.process == ProcessState::Exited {
            return ProcessExitDisposition::AlreadyExited;
        }
        if self.revision.advance().is_err() {
            return ProcessExitDisposition::AuthorityExhausted;
        }
        self.state.process = ProcessState::Exited;
        let (turn, disposition) = if self.state.final_hook_seen {
            (TurnState::Idle, ProcessExitDisposition::Confirmed)
        } else {
            self.state.instrumentation = InstrumentationState::Unavailable;
            (TurnState::Unknown, ProcessExitDisposition::Unconfirmed)
        };
        normalize_inactive_state(&mut self.state, turn);
        disposition
    }

    pub fn process_exit_with_identity(
        &mut self,
        observed: ProcessIdentity,
    ) -> Result<ProcessExitDisposition, HookViolation> {
        if self.state.revoked {
            return Ok(ProcessExitDisposition::Revoked);
        }
        if observed != self.state.expected_identity.process {
            return self.invalidate_process(observed, HookViolation::ProcessMismatch);
        }
        Ok(self.process_exit())
    }

    fn record_degraded(
        &mut self,
        envelope: HookEnvelope,
        disposition: ApplyDisposition,
    ) -> Result<ApplyDisposition, HookViolation> {
        self.revision.advance()?;
        crate::reducer::degraded::canonicalize(&mut self.state);
        crate::lifecycle_metadata::apply(&mut self.state, &envelope);
        crate::reducer::history::record(&mut self.state, envelope);
        Ok(disposition)
    }
}
