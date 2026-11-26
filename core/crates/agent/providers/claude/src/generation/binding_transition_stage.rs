use super::binding_transition::{BindingTransitionProof, PendingBindingTransition};
use super::constants::*;
use super::{ClaudeGenerationAuthority, ClaudeGenerationStore};
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};

impl ClaudeGenerationStore {
    pub(super) fn stage_binding_transition(
        &self,
        mutation: &super::mutation::GenerationMutation<'_>,
        namespace: &NamespaceLease,
        before: &super::authority_set::ClaudeInvocationAuthorities,
        predecessor: &ClaudeGenerationAuthority,
        target: &ClaudeGenerationAuthority,
        state: &mut super::authority_set::ClaudeInvocationAuthorities,
        proof: BindingTransitionProof,
    ) -> Result<bool> {
        if predecessor.binding_revision != Some(predecessor.revision)
            || state.binding_publication.is_none()
        {
            return Ok(false);
        }
        let binding_path = self.binding_path()?;
        let binding = yolo_agent_binding_io::AgentBindingFile::new(binding_path)
            .read_in_namespace(namespace)?
            .context(AUTHORITY_MISSING)?;
        ensure!(
            predecessor.binding_subject_matches(&binding)
                && predecessor.transcript_path.as_deref()
                    == Some(std::path::Path::new(&binding.evidence_path))
                && binding.process_pid == Some(predecessor.process.pid)
                && binding.process_start_time == Some(predecessor.process.start_time),
            AUTHORITY_SNAPSHOT_MISMATCH
        );
        let target_binding = target
            .transition_binding(Some(&binding))
            .context(AUTHORITY_MISSING)?;
        let predecessor_publication_generation = state.publication_generation;
        let predecessor_publication = state.binding_publication;
        state.bump_publication_preserving_transition()?;
        let transition = PendingBindingTransition::new(
            predecessor,
            target,
            state.publication_generation,
            binding,
            target_binding,
            predecessor_publication_generation,
            predecessor_publication,
            proof,
        )?;
        state.replace(predecessor, target.clone())?;
        state.pending_binding_transition = Some(transition.clone());
        transition.verify_transcript()?;
        self.exchange_set_in(mutation, namespace, before, state, || {
            transition.verify_transcript()
        })?;
        Ok(true)
    }
}
