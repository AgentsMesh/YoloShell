use super::authority_set::{ActiveBindingPublication, ClaudeInvocationAuthorities};
use super::binding_transition::PendingBindingTransition;
use super::constants::*;
use super::durable_binding_io::{read_binding, read_binding_with_identity, write_binding};
use super::{AuthorityAuthentication, ClaudeGenerationAuthority, ClaudeGenerationStore};
use crate::agent::{AgentBinding, AgentState};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};
use std::path::Path;

#[path = "durable_publication_selection.rs"]
mod selection;
#[path = "durable_publication_unstaged.rs"]
mod unstaged;

#[derive(Clone, Copy)]
struct UnstagedPublication<'a> {
    force_persist: bool,
    hook_admission: Option<&'a super::HookPublicationAdmission>,
}
impl ClaudeGenerationStore {
    pub fn publish_binding_state_in_namespace(
        &self,
        namespace: &NamespaceLease,
        binding_path: &Path,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
        force_persist: bool,
        validate: impl Fn(&AgentBinding) -> Result<()>,
    ) -> Result<(ClaudeGenerationAuthority, AgentBinding)> {
        self.publish_binding_state(
            namespace,
            binding_path,
            authentication,
            expected,
            UnstagedPublication {
                force_persist,
                hook_admission: None,
            },
            validate,
        )
    }

    pub fn publish_hook_binding_state_in_namespace(
        &self,
        namespace: &NamespaceLease,
        binding_path: &Path,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
        force_persist: bool,
        admission: &super::HookPublicationAdmission,
        validate: impl Fn(&AgentBinding) -> Result<()>,
    ) -> Result<(ClaudeGenerationAuthority, AgentBinding)> {
        self.publish_binding_state(
            namespace,
            binding_path,
            authentication,
            expected,
            UnstagedPublication {
                force_persist,
                hook_admission: Some(admission),
            },
            validate,
        )
    }

    fn publish_binding_state(
        &self,
        namespace: &NamespaceLease,
        binding_path: &Path,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
        publication: UnstagedPublication<'_>,
        validate: impl Fn(&AgentBinding) -> Result<()>,
    ) -> Result<(ClaudeGenerationAuthority, AgentBinding)> {
        let mutation = super::mutation::GenerationMutation::acquire(self, namespace)?;
        let set = self.load_set(namespace)?.context(AUTHORITY_MISSING)?;
        ensure!(
            set.active_invocation_id == authentication.invocation_id,
            AUTHORITY_AUTHENTICATION_MISMATCH
        );
        let current = set
            .authenticated(authentication)
            .cloned()
            .context(AUTHORITY_AUTHENTICATION_MISMATCH)?;
        ensure!(current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        match set.pending_binding_transition.clone() {
            Some(transition) => self.finish_pending(
                &mutation,
                namespace,
                binding_path,
                set,
                current,
                transition,
                publication,
                &validate,
            ),
            None => self.finish_unstaged(
                &mutation,
                namespace,
                binding_path,
                set,
                current,
                publication,
                &validate,
            ),
        }
    }

    fn finish_pending(
        &self,
        mutation: &super::mutation::GenerationMutation<'_>,
        namespace: &NamespaceLease,
        binding_path: &Path,
        set: ClaudeInvocationAuthorities,
        current: ClaudeGenerationAuthority,
        transition: PendingBindingTransition,
        publication: UnstagedPublication<'_>,
        validate: &impl Fn(&AgentBinding) -> Result<()>,
    ) -> Result<(ClaudeGenerationAuthority, AgentBinding)> {
        ensure!(
            transition.matches_target(&current, set.publication_generation),
            DURABLE_PENDING_INVALID
        );
        transition.verify_transcript()?;
        validate(transition.target_binding())?;
        let (existing, binding_identity) = read_binding_with_identity(namespace, binding_path)?;
        if existing.as_ref() != Some(transition.target_binding()) {
            write_binding(
                mutation,
                namespace,
                binding_path,
                &set,
                existing.as_ref(),
                binding_identity,
                publication.hook_admission,
                transition.target_binding(),
            )?;
        }
        ensure!(
            read_binding(namespace, binding_path)?.as_ref() == Some(transition.target_binding()),
            DURABLE_BINDING_INVALID
        );
        let mut next = set.clone();
        next.pending_binding_transition = None;
        if transition.target_state == AgentState::Unknown {
            next.binding_publication = None;
            next.replace(&current, current.clone())?;
        } else {
            let mut committed = current.clone();
            committed.binding_revision = Some(committed.revision);
            next.replace(&current, committed.clone())?;
            next.binding_publication = Some(ActiveBindingPublication {
                invocation_id: committed.invocation_id,
                publication_generation: next.publication_generation,
                authority_revision: committed.revision,
            });
        }
        self.exchange_set_in(mutation, namespace, &set, &next, || {
            transition.verify_transcript()?;
            if let Some(admission) = publication.hook_admission {
                admission.verify_against(next.active()?)?;
            }
            ensure!(
                read_binding(namespace, binding_path)?.as_ref()
                    == Some(transition.target_binding()),
                DURABLE_BINDING_INVALID
            );
            Ok(())
        })?;
        Ok((next.active()?.clone(), transition.target_binding().clone()))
    }
}
