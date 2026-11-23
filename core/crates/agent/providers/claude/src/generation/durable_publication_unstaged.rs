use super::super::authority_set::{ActiveBindingPublication, ClaudeInvocationAuthorities};
use super::super::constants::*;
use super::super::durable_binding_io::{read_binding, read_binding_with_identity, write_binding};
use super::super::{ClaudeGenerationAuthority, ClaudeGenerationStore};
use super::{UnstagedPublication, selection};
use crate::agent::{AgentBinding, AgentState};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};
use std::path::Path;

impl ClaudeGenerationStore {
    pub(super) fn finish_unstaged(
        &self,
        mutation: &super::super::mutation::GenerationMutation<'_>,
        namespace: &NamespaceLease,
        binding_path: &Path,
        set: ClaudeInvocationAuthorities,
        current: ClaudeGenerationAuthority,
        publication: UnstagedPublication<'_>,
        validate: &impl Fn(&AgentBinding) -> Result<()>,
    ) -> Result<(ClaudeGenerationAuthority, AgentBinding)> {
        let (existing, binding_identity) = read_binding_with_identity(namespace, binding_path)?;
        let was_published = selection::was_published(
            set.binding_publication.is_some(),
            &current,
            existing.as_ref(),
        );
        if was_published && !publication.force_persist {
            let binding = existing.context(DURABLE_BINDING_MISSING)?;
            validate(&binding)?;
            if let Some(admission) = publication.hook_admission {
                admission.verify_against(&current)?;
            }
            return Ok((current, binding));
        }
        ensure!(
            set.binding_publication.is_none() || current.binding_revision == Some(current.revision),
            DURABLE_PUBLICATION_INVALID
        );
        if set.binding_publication.is_some() {
            ensure!(
                existing
                    .as_ref()
                    .is_some_and(|binding| current.binding_subject_matches(binding)),
                DURABLE_BINDING_INVALID
            );
        }
        let admission = publication.hook_admission;
        let unmanaged_admission =
            if set.binding_publication.is_none() && current.binding_revision.is_none() {
                publication.hook_admission
            } else {
                None
            };
        let admission_accepts = unmanaged_admission
            .map(|admission| admission.accepts(existing.as_ref()))
            .transpose()?
            .unwrap_or(false);
        let target = unmanaged_admission
            .filter(|_| admission_accepts)
            .and_then(|_| current.binding(existing.as_ref()))
            .or_else(|| selection::target(&current, existing.as_ref()))
            .context(DURABLE_BINDING_MISSING)?;
        validate(&target)?;
        let (set, current) = if set.binding_publication.is_some() {
            let before = set.clone();
            let mut revoked = current.clone();
            revoked.binding_revision = None;
            let mut next = set;
            next.bump_publication()?;
            next.replace(&current, revoked.clone())?;
            self.exchange_set_in(mutation, namespace, &before, &next, || {
                if let Some(admission) = admission {
                    admission.verify_against(&current)?;
                }
                Ok(())
            })?;
            (next, revoked)
        } else {
            (set, current)
        };
        if existing.as_ref() != Some(&target) {
            write_binding(
                mutation,
                namespace,
                binding_path,
                &set,
                existing.as_ref(),
                binding_identity,
                admission,
                &target,
            )?;
        }
        ensure!(
            read_binding(namespace, binding_path)?.as_ref() == Some(&target),
            DURABLE_BINDING_INVALID
        );
        if current.state == AgentState::Unknown {
            ensure!(
                set.binding_publication.is_none() && current.binding_revision.is_none(),
                DURABLE_PUBLICATION_INVALID
            );
            return Ok((current, target));
        }
        let before = set.clone();
        let mut next = set;
        let mut committed = current.clone();
        committed.binding_revision = Some(committed.revision);
        next.replace(&current, committed.clone())?;
        next.pending_binding_transition = None;
        next.binding_publication = Some(ActiveBindingPublication {
            invocation_id: committed.invocation_id,
            publication_generation: next.publication_generation,
            authority_revision: committed.revision,
        });
        self.exchange_set_in(mutation, namespace, &before, &next, || {
            ensure!(
                read_binding(namespace, binding_path)?.as_ref() == Some(&target),
                DURABLE_BINDING_INVALID
            );
            if let Some(admission) = admission {
                admission.verify_against(&committed)?;
            }
            Ok(())
        })?;
        Ok((committed, target))
    }
}
