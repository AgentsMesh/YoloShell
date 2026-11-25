use super::authority_set::{ActiveBindingPublication, ClaudeInvocationAuthorities};
use super::constants::*;
use super::durable_binding_io::read_binding_with_identity;
use super::{AuthorityAuthentication, ClaudeGenerationAuthority, ClaudeGenerationStore};
use crate::agent::{AgentRecoveryAvailability, AgentState};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};

#[path = "process_exit_validation.rs"]
mod exit_validation;
use exit_validation::{ensure_process_gone, final_candidate, process_is_gone, same_process};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessExitDisposition {
    Confirmed,
    AlreadyExited,
    NotCandidate,
    Unconfirmed,
}

impl ClaudeGenerationStore {
    pub fn finalize_process_exit_in_namespace(
        &self,
        namespace: &NamespaceLease,
        authentication: AuthorityAuthentication,
        expected: &ClaudeGenerationAuthority,
        process: crate::process::ProcessIdentity,
    ) -> Result<ProcessExitDisposition> {
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
        ensure!(
            same_process(&current, process),
            PROCESS_EXIT_IDENTITY_MISMATCH
        );
        if !process_is_gone(process) {
            return Ok(ProcessExitDisposition::Unconfirmed);
        }
        if current.state == AgentState::Exited {
            return self.publish_exited(&mutation, namespace, set, current, process);
        }
        if !final_candidate(&current) {
            return Ok(ProcessExitDisposition::NotCandidate);
        }
        if current.state != AgentState::Unknown
            || set.pending_binding_transition.is_some()
            || set.binding_publication.is_some()
            || current.binding_revision.is_some()
        {
            return Ok(ProcessExitDisposition::Unconfirmed);
        }
        let binding_path = self.binding_path()?;
        let (existing, _) = read_binding_with_identity(namespace, &binding_path)?;
        if !exit_validation::unknown_binding_matches(&current, existing.as_ref()) {
            return Ok(ProcessExitDisposition::Unconfirmed);
        }
        let next_authority = super::transition_mutation::process_exit(
            &current,
            crate::agent::claude::generation::AuthorityProcessIdentity {
                pid: process.pid,
                start_time: process.start_time,
            },
        )?;
        let before = set.clone();
        let mut staged = set;
        staged.bump_publication()?;
        staged.replace(&current, next_authority)?;
        self.exchange_set_in(&mutation, namespace, &before, &staged, || {
            ensure_process_gone(process)
        })?;
        let current = staged.active()?.clone();
        self.publish_exited(&mutation, namespace, staged, current, process)
    }

    fn publish_exited(
        &self,
        mutation: &super::mutation::GenerationMutation<'_>,
        namespace: &NamespaceLease,
        set: ClaudeInvocationAuthorities,
        current: ClaudeGenerationAuthority,
        process: crate::process::ProcessIdentity,
    ) -> Result<ProcessExitDisposition> {
        let binding_path = self.binding_path()?;
        let (existing, binding_identity) = read_binding_with_identity(namespace, &binding_path)?;
        if set.binding_publication.is_some() && current.binding_revision == Some(current.revision) {
            return if existing
                .as_ref()
                .is_some_and(|binding| current.matches_binding(binding))
            {
                Ok(ProcessExitDisposition::AlreadyExited)
            } else {
                Ok(ProcessExitDisposition::Unconfirmed)
            };
        }
        if set.pending_binding_transition.is_some()
            || set.binding_publication.is_some()
            || current.binding_revision.is_some()
        {
            return Ok(ProcessExitDisposition::Unconfirmed);
        }
        let Some(existing) = existing else {
            return Ok(ProcessExitDisposition::Unconfirmed);
        };
        let Some(target) = current.transition_binding(Some(&existing)) else {
            return Ok(ProcessExitDisposition::Unconfirmed);
        };
        ensure!(
            target.state == AgentState::Exited
                && target.recovery_availability == AgentRecoveryAvailability::Available
                && target.process_pid.is_none()
                && target.process_start_time.is_none()
                && current.matches_binding_payload(&target),
            DURABLE_BINDING_INVALID
        );
        if existing != target {
            ensure!(
                exit_validation::unknown_binding_matches(&current, Some(&existing)),
                DURABLE_BINDING_INVALID
            );
            mutation.write_path(
                namespace,
                binding_path.as_path(),
                binding_identity,
                &target,
                || {
                    ensure_process_gone(process)?;
                    ensure!(
                        self.load_set(namespace)?.as_ref() == Some(&set),
                        AUTHORITY_SNAPSHOT_MISMATCH
                    );
                    Ok(())
                },
            )?;
        }
        ensure!(
            super::durable_binding_io::read_binding(namespace, &binding_path)?.as_ref()
                == Some(&target),
            DURABLE_BINDING_INVALID
        );
        let mut committed = current.clone();
        committed.binding_revision = Some(committed.revision);
        let mut next = set.clone();
        next.replace(&current, committed.clone())?;
        next.pending_binding_transition = None;
        next.binding_publication = Some(ActiveBindingPublication {
            invocation_id: committed.invocation_id,
            publication_generation: next.publication_generation,
            authority_revision: committed.revision,
        });
        self.exchange_set_in(mutation, namespace, &set, &next, || {
            ensure_process_gone(process)?;
            ensure!(
                super::durable_binding_io::read_binding(namespace, &binding_path)?.as_ref()
                    == Some(&target),
                DURABLE_BINDING_INVALID
            );
            Ok(())
        })?;
        Ok(ProcessExitDisposition::Confirmed)
    }
}
