#[cfg(any(test, feature = "adversarial-test-hooks"))]
use super::authority_set::ActiveBindingPublication;
use super::constants::*;
use super::{AuthorityAuthentication, ClaudeGenerationAuthority, ClaudeGenerationStore};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};

impl ClaudeGenerationStore {
    pub fn binding_is_published_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
    ) -> Result<bool> {
        let state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        ensure!(
            state.active_invocation_id == authentication.invocation_id,
            AUTHORITY_AUTHENTICATION_MISMATCH
        );
        let current = state
            .authenticated(authentication)
            .context(AUTHORITY_AUTHENTICATION_MISMATCH)?;
        ensure!(current == expected, AUTHORITY_SNAPSHOT_MISMATCH);
        if let Some(pending) = state.pending_binding_transition.as_ref() {
            ensure!(
                pending.matches_target(current, state.publication_generation),
                AUTHORITY_PUBLICATION_STATE_INVALID
            );
            pending.verify_transcript()?;
            return Ok(false);
        }
        Ok(state.binding_publication.is_some()
            && current.binding_revision == Some(current.revision))
    }

    pub fn fence_binding_if_active_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
    ) -> Result<()> {
        let mutation = super::mutation::GenerationMutation::acquire(self, namespace)?;
        let mut state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        let current = state
            .authenticated(authentication)
            .cloned()
            .context(AUTHORITY_AUTHENTICATION_MISMATCH)?;
        ensure!(current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        if state.pending_binding_transition.is_some() {
            let before = state.clone();
            state.pending_binding_transition = None;
            state.bump_publication()?;
            state.replace(expected, current)?;
            return self.exchange_set_in(&mutation, namespace, &before, &state, || Ok(()));
        }
        if state.active_invocation_id != current.invocation_id
            || (state.binding_publication.is_none() && current.binding_revision.is_none())
        {
            return Ok(());
        }
        let before = state.clone();
        state.bump_publication()?;
        let mut pending = current;
        pending.binding_revision = None;
        state.replace(expected, pending)?;
        self.exchange_set_in(&mutation, namespace, &before, &state, || Ok(()))
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn begin_binding_publication_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
    ) -> Result<ClaudeGenerationAuthority> {
        let mutation = super::mutation::GenerationMutation::acquire(self, namespace)?;
        let mut state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        ensure!(
            state.pending_binding_transition.is_none(),
            AUTHORITY_PUBLICATION_STATE_INVALID
        );
        ensure!(
            state.active_invocation_id == authentication.invocation_id,
            AUTHORITY_AUTHENTICATION_MISMATCH
        );
        let current = state
            .authenticated(authentication)
            .cloned()
            .context(AUTHORITY_AUTHENTICATION_MISMATCH)?;
        ensure!(current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        if state.binding_publication.is_none() && current.binding_revision.is_none() {
            return Ok(current);
        }
        let before = state.clone();
        state.bump_publication()?;
        let mut pending = current;
        pending.binding_revision = None;
        state.replace(expected, pending.clone())?;
        self.exchange_set_in(&mutation, namespace, &before, &state, || Ok(()))?;
        Ok(pending)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn commit_binding_publication_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
    ) -> Result<ClaudeGenerationAuthority> {
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
        if let Some(transition) = state.pending_binding_transition.clone() {
            ensure!(
                transition.matches_target(&current, state.publication_generation),
                AUTHORITY_PUBLICATION_STATE_INVALID
            );
            transition.verify_transcript()?;
            let binding_path = self.binding_path()?;
            let existing = super::durable_binding_io::read_binding(namespace, &binding_path)?;
            ensure!(
                existing.as_ref() == Some(transition.target_binding()),
                AUTHORITY_PUBLICATION_STATE_INVALID
            );
            let before = state.clone();
            let mut committed = current;
            committed.binding_revision = Some(committed.revision);
            state.replace(expected, committed.clone())?;
            state.pending_binding_transition = None;
            state.binding_publication = Some(ActiveBindingPublication {
                invocation_id: committed.invocation_id,
                publication_generation: state.publication_generation,
                authority_revision: committed.revision,
            });
            self.exchange_set_in(&mutation, namespace, &before, &state, || Ok(()))?;
            return Ok(committed);
        }
        ensure!(
            state.pending_binding_transition.is_none(),
            AUTHORITY_PUBLICATION_STATE_INVALID
        );
        ensure!(
            current.binding_revision.is_none() && state.binding_publication.is_none(),
            AUTHORITY_PUBLICATION_STATE_INVALID
        );
        let before = state.clone();
        let mut committed = current;
        committed.binding_revision = Some(committed.revision);
        state.replace(expected, committed.clone())?;
        state.binding_publication = Some(ActiveBindingPublication {
            invocation_id: committed.invocation_id,
            publication_generation: state.publication_generation,
            authority_revision: committed.revision,
        });
        self.exchange_set_in(&mutation, namespace, &before, &state, || Ok(()))?;
        Ok(committed)
    }
}
