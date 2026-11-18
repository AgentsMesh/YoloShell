use super::ClaudeGenerationStore;
use super::authority_set::ClaudeInvocationAuthorities;
use super::constants::*;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
use super::{
    AuthorityAuthentication, ClaudeGenerationAuthority, GenerationApplyDisposition, GenerationEvent,
};
use crate::namespace_lease::NamespaceLease;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
use anyhow::ensure;
use anyhow::{Context, Result};

impl ClaudeGenerationStore {
    pub(super) fn load_set(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Option<ClaudeInvocationAuthorities>> {
        if namespace.file_is_missing(&self.path)? {
            return Ok(None);
        }
        let state: ClaudeInvocationAuthorities = namespace
            .read_json(&self.path)
            .context(AUTHORITY_LOAD_CONTEXT)?;
        state.validate()?;
        Ok(Some(state))
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) fn authenticated(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
    ) -> Result<ClaudeGenerationAuthority> {
        let state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        ensure!(
            state.active_invocation_id == authentication.invocation_id,
            AUTHORITY_AUTHENTICATION_MISMATCH
        );
        state
            .authenticated(authentication)
            .cloned()
            .context(AUTHORITY_AUTHENTICATION_MISMATCH)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(crate) fn apply_expected_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
        sequence: u64,
        event: &GenerationEvent,
        transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
    ) -> Result<(ClaudeGenerationAuthority, GenerationApplyDisposition)> {
        let current = self.authenticated(namespace, authentication)?;
        ensure!(current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        let (next, disposition) =
            super::transition::apply(&current, sequence, event, transcript_identity)?;
        if disposition == GenerationApplyDisposition::Applied {
            self.exchange_authority(namespace, &current, next.clone())?;
        }
        Ok((next, disposition))
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) fn exchange_authority(
        &self,
        namespace: &NamespaceLease,
        expected: &ClaudeGenerationAuthority,
        next: ClaudeGenerationAuthority,
    ) -> Result<()> {
        let mut state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        ensure!(
            state.active_invocation_id == expected.invocation_id && next.binding_revision.is_none(),
            AUTHORITY_PUBLICATION_STATE_INVALID
        );
        let before = state.clone();
        state.bump_publication()?;
        state.replace(expected, next)?;
        self.exchange_set(namespace, &before, &state)
    }
}
