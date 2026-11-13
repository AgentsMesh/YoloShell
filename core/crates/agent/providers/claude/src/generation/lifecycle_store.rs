use super::binding_transition::BindingTransitionProof;
use super::constants::*;
use super::{
    AuthorityAuthentication, ClaudeGenerationAuthority, ClaudeGenerationStore,
    GenerationApplyDisposition, GenerationEvent,
};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};

#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../../test_support/generation/lifecycle_store_api.rs"]
mod test_api;

impl ClaudeGenerationStore {
    pub fn plan_invocation_transition(
        &self,
        expected: &ClaudeGenerationAuthority,
        sequence: u64,
        event: &GenerationEvent,
        transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
    ) -> Result<(ClaudeGenerationAuthority, GenerationApplyDisposition)> {
        super::transition::apply(expected, sequence, event, transcript_identity)
    }

    pub fn preview_invocation_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        sequence: u64,
        event: &GenerationEvent,
    ) -> Result<(ClaudeGenerationAuthority, GenerationApplyDisposition)> {
        let state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        let current = state
            .authenticated(authentication)
            .cloned()
            .context(AUTHORITY_AUTHENTICATION_MISMATCH)?;
        let disposition = super::transition::preview(&current, sequence, event)?;
        Ok((current, disposition))
    }

    pub fn transition_foreground_durable_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
        sequence: u64,
        event: &GenerationEvent,
        transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
    ) -> Result<(ClaudeGenerationAuthority, GenerationApplyDisposition, bool)> {
        self.transition_in_namespace(
            namespace,
            authentication,
            expected,
            sequence,
            event,
            transcript_identity,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn transition_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
        sequence: u64,
        event: &GenerationEvent,
        transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
        durable_binding: bool,
    ) -> Result<(ClaudeGenerationAuthority, GenerationApplyDisposition, bool)> {
        let mutation = super::mutation::GenerationMutation::acquire(self, namespace)?;
        let mut state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        let current = state
            .authenticated(authentication)
            .cloned()
            .context(AUTHORITY_AUTHENTICATION_MISMATCH)?;
        ensure!(current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        let (next, disposition) =
            super::transition::apply(&current, sequence, event, transcript_identity)?;
        if disposition == GenerationApplyDisposition::Invalidated {
            return Ok((current, disposition, false));
        }
        if let Some(pending) = state.pending_binding_transition.as_ref() {
            ensure!(
                pending.matches_target(&current, state.publication_generation),
                AUTHORITY_PUBLICATION_STATE_INVALID
            );
            return Ok((current, disposition, false));
        }
        let before = state.clone();
        let mut staged = false;
        if disposition == GenerationApplyDisposition::Applied {
            if durable_binding
                && state.active_invocation_id == current.invocation_id
                && matches!(
                    event,
                    GenerationEvent::StateChanged { .. } | GenerationEvent::SessionEnd { .. }
                )
            {
                staged = self.stage_binding_transition(
                    &mutation,
                    namespace,
                    &before,
                    &current,
                    &next,
                    &mut state,
                    BindingTransitionProof::Event {
                        sequence,
                        event: event.clone(),
                        transcript_identity,
                    },
                )?;
            }
            if !staged {
                state.replace(&current, next)?;
            }
        }
        let reactivated = state.activate(current.invocation_id)?;
        if disposition == GenerationApplyDisposition::Applied && !reactivated && !staged {
            state.bump_publication()?;
        }
        if state != before && !staged {
            self.exchange_set_in(&mutation, namespace, &before, &state, || Ok(()))?;
        }
        let authority = state
            .authority(current.invocation_id)
            .cloned()
            .context(AUTHORITY_MISSING)?;
        Ok((authority, disposition, reactivated))
    }
}
