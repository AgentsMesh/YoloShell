use super::constants::*;
use super::{AuthorityRuntimeIdentity, ClaudeGenerationAuthority, ClaudeGenerationStore};
use crate::agent::{AgentAdapter, AgentState, ObservedProcess};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};

impl ClaudeGenerationStore {
    pub fn foreground_invocation_in_namespace(
        &self,
        namespace: &NamespaceLease,
        runtime: AuthorityRuntimeIdentity,
        processes: &[ObservedProcess],
    ) -> Result<Option<(ClaudeGenerationAuthority, bool)>> {
        let Some(state) = self.load_set(namespace)? else {
            return Ok(None);
        };
        let mut matches = state.authorities.iter().filter(|authority| {
            authority.matches_runtime(runtime)
                && matches!(
                    authority.state,
                    AgentState::Running | AgentState::WaitingForInput
                )
                && processes.iter().any(|process| {
                    process.identity_matches(authority.process.pid, authority.process.start_time)
                        && process.is_foreground()
                        && process.cwd.as_deref() == Some(authority.cwd.as_path())
                        && crate::agent::claude::ClaudeAdapter.matches_process(process)
                })
        });
        let Some(authority) = matches.next() else {
            return Ok(None);
        };
        ensure!(matches.next().is_none(), AUTHORITY_FOREGROUND_AMBIGUOUS);
        Ok(Some((
            authority.clone(),
            state.active_invocation_id == authority.invocation_id,
        )))
    }

    pub fn activate_observed_in_namespace(
        &self,
        namespace: &NamespaceLease,
        expected: &ClaudeGenerationAuthority,
        observed_state: AgentState,
    ) -> Result<ClaudeGenerationAuthority> {
        let mutation = super::mutation::GenerationMutation::acquire(self, namespace)?;
        ensure!(
            matches!(
                observed_state,
                AgentState::Running | AgentState::WaitingForInput
            ),
            AUTHORITY_EVENT_INVALID
        );
        let mut state = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        let current = state
            .authority(expected.invocation_id)
            .cloned()
            .context(AUTHORITY_MISSING)?;
        ensure!(current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        let before = state.clone();
        let state_changed = current.state != observed_state;
        if state_changed {
            let next = super::transition_mutation::advance_state(&current, observed_state)?;
            state.replace(&current, next)?;
        }
        let reactivated = state.activate(current.invocation_id)?;
        if state_changed && !reactivated {
            state.bump_publication()?;
        }
        if state != before {
            self.exchange_set_in(&mutation, namespace, &before, &state, || Ok(()))?;
        }
        state
            .authority(current.invocation_id)
            .cloned()
            .context(AUTHORITY_MISSING)
    }
}
