use super::*;
use crate::namespace_lease::NamespaceLease;
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};

impl ClaudeNativeGenerationStore {
    pub fn revoke_all_with_guard(
        &self,
        guard: &SessionDurableMutationGuard<'_>,
        expected: &ClaudeNativeGeneration,
        expected_identity: FileIdentity,
        mut fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        ensure!(
            guard.terminal_id() == expected.scope.terminal_id,
            INVALID
        );
        let (mut state, identity) = self.load(guard.namespace())?.context(INVALID)?;
        ensure!(
            identity == expected_identity
                && state.active_invocation == expected.invocation_id
                && state.invocations.iter().any(|record| record == expected),
            INVALID
        );
        for record in state.invocations.clone() {
            revoke(&mut state, &record)?;
        }
        guard.write_json_expected_with_fence(&self.path, Some(identity), &state, |_| fence())
    }

    pub fn revoke_in_namespace(
        &self,
        namespace: &NamespaceLease,
        expected: &ClaudeNativeGeneration,
        mut fence: impl FnMut() -> Result<()>,
    ) -> Result<ClaudeNativeGeneration> {
        let mutation =
            super::super::mutation::GenerationMutation::acquire(&self.generation, namespace)?;
        let (mut state, identity) = self.load(namespace)?.context(INVALID)?;
        let result = revoke(&mut state, expected)?;
        mutation.write_path(namespace, &self.path, Some(identity), &state, || fence())?;
        Ok(result)
    }

    pub fn revoke_with_guard(
        &self,
        guard: &SessionDurableMutationGuard<'_>,
        expected: &ClaudeNativeGeneration,
        expected_identity: FileIdentity,
        mut fence: impl FnMut() -> Result<()>,
    ) -> Result<ClaudeNativeGeneration> {
        ensure!(
            guard.terminal_id() == expected.scope.terminal_id,
            INVALID
        );
        let (mut state, identity) = self.load(guard.namespace())?.context(INVALID)?;
        ensure!(identity == expected_identity, INVALID);
        let result = revoke(&mut state, expected)?;
        guard.write_json_expected_with_fence(&self.path, Some(identity), &state, |_| fence())?;
        Ok(result)
    }
}

fn revoke(
    state: &mut NativeGenerations,
    expected: &ClaudeNativeGeneration,
) -> Result<ClaudeNativeGeneration> {
    let record = state
        .invocations
        .iter_mut()
        .find(|item| item.invocation_id == expected.invocation_id)
        .context(INVALID)?;
    ensure!(record == expected, INVALID);
    if !record.revoked {
        record.revoked = true;
        record.activity = AgentState::Unknown;
        record.binding.state = AgentState::Unknown;
        record.binding.recovery_availability = crate::agent::AgentRecoveryAvailability::Unavailable;
        record.revision = record.revision.checked_add(INCREMENT).context(CAPACITY)?;
    }
    let result = record.clone();
    state.validate()?;
    Ok(result)
}
