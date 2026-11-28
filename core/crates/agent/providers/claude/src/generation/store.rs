use super::authority_set::ClaudeInvocationAuthorities;
use super::constants::*;
use super::record;
use super::types::{ClaudeGenerationAuthority, GenerationBegin};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/generation/store_test_api.rs"]
mod test_api;

#[derive(Clone, Debug)]
pub struct ClaudeGenerationStore {
    pub(crate) path: PathBuf,
}

impl ClaudeGenerationStore {
    pub fn from_binding_path(binding_path: &Path) -> Result<Self> {
        let parent = binding_path.parent().context(BINDING_PARENT_MISSING)?;
        Ok(Self {
            path: parent.join(AUTHORITY_FILE_NAME),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(test)]
    pub(crate) fn publication_generation_in_namespace(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<u64> {
        Ok(self
            .load_set(namespace)?
            .context(AUTHORITY_MISSING)?
            .publication_generation)
    }

    pub fn load_in_namespace(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Option<ClaudeGenerationAuthority>> {
        self.load_set(namespace)?
            .map(|state| state.active().cloned())
            .transpose()
    }

    pub fn load_invocation_in_namespace(
        &self,
        namespace: &NamespaceLease,
        invocation_id: uuid::Uuid,
    ) -> Result<Option<ClaudeGenerationAuthority>> {
        Ok(self
            .load_set(namespace)?
            .and_then(|state| state.authority(invocation_id).cloned()))
    }

    pub fn begin_foreground_in_namespace(
        &self,
        namespace: &NamespaceLease,
        begin: &GenerationBegin,
        active_is_foreground: bool,
    ) -> Result<ClaudeGenerationAuthority> {
        super::validation::validate_begin(begin)?;
        let mutation = super::mutation::GenerationMutation::acquire(self, namespace)?;
        let Some(mut state) = self.load_set(namespace)? else {
            let authority = record::begin(None, begin)?;
            mutation
                .write(
                    self,
                    namespace,
                    None,
                    &ClaudeInvocationAuthorities::new(authority.clone()),
                    || Ok(()),
                )
                .context(AUTHORITY_CREATE_CONTEXT)?;
            return Ok(authority);
        };
        if let Some(authority) = state.matching_begin(begin).cloned() {
            let expected = state.clone();
            if state.activate(authority.invocation_id)? {
                ensure!(!active_is_foreground, AUTHORITY_LIVE_RESERVATION);
                self.exchange_set_in(&mutation, namespace, &expected, &state, || Ok(()))?;
            }
            return Ok(authority);
        }
        ensure!(
            state.terminal_id == begin.terminal_id,
            AUTHORITY_SESSION_MISMATCH
        );
        ensure!(!active_is_foreground, AUTHORITY_LIVE_RESERVATION);
        let expected = state.clone();
        ensure!(
            state.authorities.len() < MAX_INVOCATION_AUTHORITIES,
            AUTHORITY_SET_CAPACITY
        );
        let authority = record::begin(None, begin)?;
        state.authorities.push_back(authority.clone());
        state.activate(authority.invocation_id)?;
        state.validate()?;
        self.exchange_set_in(&mutation, namespace, &expected, &state, || Ok(()))?;
        Ok(authority)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) fn exchange_set(
        &self,
        namespace: &NamespaceLease,
        expected: &ClaudeInvocationAuthorities,
        next: &ClaudeInvocationAuthorities,
    ) -> Result<()> {
        self.exchange_set_with_commit_fence(namespace, expected, next, || Ok(()))
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) fn exchange_set_with_commit_fence<F>(
        &self,
        namespace: &NamespaceLease,
        expected: &ClaudeInvocationAuthorities,
        next: &ClaudeInvocationAuthorities,
        commit_fence: F,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        let mutation = super::mutation::GenerationMutation::acquire(self, namespace)?;
        self.exchange_set_in(&mutation, namespace, expected, next, commit_fence)
    }

    pub(super) fn exchange_set_in<F>(
        &self,
        mutation: &super::mutation::GenerationMutation<'_>,
        namespace: &NamespaceLease,
        expected: &ClaudeInvocationAuthorities,
        next: &ClaudeInvocationAuthorities,
        commit_fence: F,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        next.validate()?;
        let (current, identity): (ClaudeInvocationAuthorities, _) = namespace
            .read_json_with_identity(&self.path)
            .context(AUTHORITY_COMPARE_EXCHANGE_CONTEXT)?;
        ensure!(current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        mutation
            .write(self, namespace, Some(identity), next, commit_fence)
            .context(AUTHORITY_COMPARE_EXCHANGE_CONTEXT)
    }
}
