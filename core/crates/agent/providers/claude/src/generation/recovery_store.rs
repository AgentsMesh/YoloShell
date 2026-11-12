use super::binding_transition::BindingTransitionProof;
use super::constants::*;
use super::recovery_admission::{process_gone, require_unreferenced};
use super::{
    AuthorityAuthentication, ClaudeGenerationAuthority, ClaudeGenerationStore,
    GenerationApplyDisposition, GenerationRecoveryEvent,
};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};

impl ClaudeGenerationStore {
    pub fn dead_parked_in_namespace(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Vec<ClaudeGenerationAuthority>> {
        let Some(state) = self.load_set(namespace)? else {
            return Ok(Vec::new());
        };
        let mut parked = state
            .authorities
            .iter()
            .filter(|authority| {
                authority.invocation_id != state.active_invocation_id && process_gone(authority)
            })
            .cloned()
            .collect::<Vec<_>>();
        if parked.is_empty() {
            return Ok(parked);
        }
        let binding = super::durable_binding_io::read_binding(namespace, &self.binding_path()?)?;
        parked.retain(|authority| {
            !binding
                .as_ref()
                .is_some_and(|value| authority.matches_binding(value))
        });
        Ok(parked)
    }

    pub fn preview_recovery(
        &self,
        current: &ClaudeGenerationAuthority,
        events: &[GenerationRecoveryEvent],
    ) -> Result<(ClaudeGenerationAuthority, bool)> {
        fold(current, events)
    }

    pub fn apply_recovery_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
        events: &[GenerationRecoveryEvent],
    ) -> Result<(ClaudeGenerationAuthority, bool)> {
        let mutation = super::mutation::GenerationMutation::acquire(self, namespace)?;
        let mut state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        ensure!(
            state.active_invocation_id == authentication.invocation_id,
            AUTHORITY_AUTHENTICATION_MISMATCH
        );
        let current = state
            .authenticated(authentication)
            .cloned()
            .context(AUTHORITY_AUTHENTICATION_MISMATCH)?;
        ensure!(current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        let (next, applied) = fold(&current, events)?;
        if !applied {
            return Ok((current, false));
        }
        let before = state.clone();
        let can_stage = state.pending_binding_transition.is_none()
            && current.binding_revision == Some(current.revision)
            && state.binding_publication.is_some()
            && !namespace.file_is_missing(&self.binding_path()?)?;
        let staged = can_stage
            && self.stage_binding_transition(
                &mutation,
                namespace,
                &before,
                &current,
                &next,
                &mut state,
                BindingTransitionProof::Recovery {
                    events: events.to_vec(),
                },
            )?;
        if !staged {
            state.bump_publication()?;
            state.replace(&current, next.clone())?;
            self.exchange_set_in(&mutation, namespace, &before, &state, || Ok(()))?;
        }
        Ok((next, true))
    }

    pub fn apply_parked_recovery_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
        events: &[GenerationRecoveryEvent],
    ) -> Result<(ClaudeGenerationAuthority, bool)> {
        let mutation = super::mutation::GenerationMutation::acquire(self, namespace)?;
        let mut state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        ensure!(
            state.active_invocation_id != authentication.invocation_id && process_gone(expected),
            AUTHORITY_PARKED_RECOVERY_INVALID
        );
        let current = state
            .authenticated(authentication)
            .cloned()
            .context(AUTHORITY_AUTHENTICATION_MISMATCH)?;
        ensure!(current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        require_unreferenced(self, namespace, &current)?;
        let (next, applied) = fold(&current, events)?;
        if !applied {
            return Ok((current, false));
        }
        let before = state.clone();
        state.replace(&current, next.clone())?;
        self.exchange_set_in(&mutation, namespace, &before, &state, || Ok(()))?;
        Ok((next, true))
    }

    pub fn retire_parked_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
    ) -> Result<()> {
        let mutation = super::mutation::GenerationMutation::acquire(self, namespace)?;
        let mut state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        ensure!(
            state.active_invocation_id != authentication.invocation_id && process_gone(expected),
            AUTHORITY_PARKED_RECOVERY_INVALID
        );
        let current = state
            .authenticated(authentication)
            .cloned()
            .context(AUTHORITY_AUTHENTICATION_MISMATCH)?;
        ensure!(current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        require_unreferenced(self, namespace, &current)?;
        let before = state.clone();
        state
            .authorities
            .retain(|authority| authority.invocation_id != expected.invocation_id);
        self.exchange_set_in(&mutation, namespace, &before, &state, || Ok(()))
    }
}

pub(super) fn fold_recovery(
    current: &ClaudeGenerationAuthority,
    events: &[GenerationRecoveryEvent],
) -> Result<(ClaudeGenerationAuthority, bool)> {
    ensure!(!events.is_empty(), AUTHORITY_RECOVERY_INVALID);
    let mut next = current.clone();
    let mut changed = false;
    for pending in events {
        let (candidate, disposition) = super::transition::apply(
            &next,
            pending.sequence,
            &pending.event,
            pending.transcript_identity,
        )?;
        match disposition {
            GenerationApplyDisposition::Applied => {
                next = candidate;
                changed = true;
            }
            GenerationApplyDisposition::Replay | GenerationApplyDisposition::Stale => {
                ensure!(exact_replay(&next, pending), AUTHORITY_RECOVERY_INVALID);
            }
            GenerationApplyDisposition::Invalidated => {
                anyhow::bail!(AUTHORITY_RECOVERY_INVALID);
            }
        }
    }
    Ok((next, changed))
}

fn fold(
    current: &ClaudeGenerationAuthority,
    events: &[GenerationRecoveryEvent],
) -> Result<(ClaudeGenerationAuthority, bool)> {
    fold_recovery(current, events)
}

fn exact_replay(authority: &ClaudeGenerationAuthority, pending: &GenerationRecoveryEvent) -> bool {
    authority
        .replay_events
        .iter()
        .any(|applied| applied.sequence == pending.sequence && applied.event == pending.event)
}
