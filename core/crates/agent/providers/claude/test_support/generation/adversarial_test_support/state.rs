use super::super::ClaudeGenerationStore;
use super::super::binding_transition::{BindingTransitionProof, PendingBindingTransition};
use super::super::constants::*;
use super::state_binding::{AUTHORITY_PENDING_MISMATCH, binding_for_state};
use crate::agent::{AgentBinding, AgentState};
use anyhow::{Context, Result, ensure};
use std::path::Path;

const BINDING_MISSING: &str = "Claude durable state binding is missing";
const FINAL_PUBLICATION_MISMATCH: &str = "Claude durable state final publication is inconsistent";
const NAMESPACE_LABEL: &str = "Claude durable state test";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PersistenceBoundary {
    AuthorityPending,
    BindingWritten,
}

pub(crate) fn persist_durable_state(binding_path: &Path, state: AgentState) -> Result<()> {
    persist(binding_path, state, |_| Ok(()))
}

#[cfg(test)]
pub(crate) fn persist_durable_state_with_boundary(
    binding_path: &Path,
    state: AgentState,
    boundary: impl FnMut(PersistenceBoundary) -> Result<()>,
) -> Result<()> {
    persist(binding_path, state, boundary)
}

#[cfg(test)]
pub(crate) fn prepare_durable_state_for_test(
    binding_path: &Path,
    state: AgentState,
) -> Result<super::super::ClaudeGenerationAuthority> {
    let session = binding_path.parent().context(BINDING_PARENT_MISSING)?;
    let lease = crate::namespace_lease::NamespaceLease::capture(&[(NAMESPACE_LABEL, session)])?;
    let store = ClaudeGenerationStore::from_binding_path(binding_path)?;
    let mutation = super::super::mutation::GenerationMutation::acquire(&store, &lease)?;
    let binding = yolo_agent_binding_io::AgentBindingFile::new(binding_path)
        .read_in_namespace(&lease)?
        .context(BINDING_MISSING)?;
    prepare(&store, &mutation, &lease, binding, state)?.context(AUTHORITY_PENDING_MISMATCH)?;
    store.load_in_namespace(&lease)?.context(AUTHORITY_MISSING)
}

fn persist(
    binding_path: &Path,
    state: AgentState,
    mut boundary: impl FnMut(PersistenceBoundary) -> Result<()>,
) -> Result<()> {
    let session = binding_path.parent().context(BINDING_PARENT_MISSING)?;
    let lease = crate::namespace_lease::NamespaceLease::capture(&[(NAMESPACE_LABEL, session)])?;
    let store = ClaudeGenerationStore::from_binding_path(binding_path)?;
    let mutation = super::super::mutation::GenerationMutation::acquire(&store, &lease)?;
    let binding = yolo_agent_binding_io::AgentBindingFile::new(binding_path)
        .read_in_namespace(&lease)?
        .context(BINDING_MISSING)?;
    let transition = prepare(&store, &mutation, &lease, binding, state)?;
    let Some(transition) = transition else {
        return Ok(());
    };
    boundary(PersistenceBoundary::AuthorityPending)?;
    install_binding(&store, &mutation, &lease, binding_path, &transition)?;
    boundary(PersistenceBoundary::BindingWritten)?;
    commit(&store, &mutation, &lease, binding_path, transition)
}

fn prepare(
    store: &ClaudeGenerationStore,
    mutation: &super::super::mutation::GenerationMutation<'_>,
    lease: &crate::namespace_lease::NamespaceLease,
    binding: AgentBinding,
    state: AgentState,
) -> Result<Option<PendingBindingTransition>> {
    let mut set = store.load_set(lease)?.context(AUTHORITY_MISSING)?;
    if let Some(transition) = set.pending_binding_transition.clone() {
        ensure!(transition.targets(state), AUTHORITY_PENDING_MISMATCH);
        ensure!(
            transition.accepts_binding(&binding),
            AUTHORITY_PENDING_MISMATCH
        );
        transition.verify_transcript()?;
        return Ok(Some(transition));
    }
    let predecessor = set.active()?.clone();
    super::state_validation::validate_binding_identity(&predecessor, &binding)?;
    if predecessor.state == state
        && (predecessor.matches_binding(&binding)
            || (state == AgentState::Unknown && binding.state == AgentState::Unknown))
    {
        return Ok(None);
    }
    let before = set.clone();
    let mut target = super::super::transition_mutation::advance_state(&predecessor, state)?;
    if state == AgentState::Unknown {
        target.transcript_path = None;
        target.transcript_identity = None;
    }
    let target_binding = binding_for_state(&target, binding.clone(), state)?;
    set.bump_publication_preserving_transition()?;
    let transition = PendingBindingTransition::new(
        &predecessor,
        &target,
        set.publication_generation,
        binding,
        target_binding,
        before.publication_generation,
        before.binding_publication,
        BindingTransitionProof::StateMutation {
            target_state: state,
        },
    )?;
    set.replace(&predecessor, target)?;
    set.pending_binding_transition = Some(transition.clone());
    transition.verify_transcript()?;
    store.exchange_set_in(mutation, lease, &before, &set, || {
        transition.verify_transcript()
    })?;
    Ok(Some(transition))
}

fn install_binding(
    store: &ClaudeGenerationStore,
    mutation: &super::super::mutation::GenerationMutation<'_>,
    lease: &crate::namespace_lease::NamespaceLease,
    binding_path: &Path,
    transition: &PendingBindingTransition,
) -> Result<()> {
    let current: AgentBinding = lease.read_json(binding_path)?;
    ensure!(
        transition.accepts_binding(&current),
        AUTHORITY_PENDING_MISMATCH
    );
    mutation.write_path(
        lease,
        binding_path,
        lease
            .read_json_with_identity::<AgentBinding>(binding_path)
            .map(|(_, identity)| Some(identity))?,
        transition.target_binding(),
        || {
            let set = store.load_set(lease)?.context(AUTHORITY_MISSING)?;
            ensure!(
                set.pending_binding_transition.as_ref() == Some(transition),
                AUTHORITY_PENDING_MISMATCH
            );
            ensure!(
                transition.matches_target(set.active()?, set.publication_generation),
                AUTHORITY_PENDING_MISMATCH
            );
            transition.verify_transcript()
        },
    )
}

fn commit(
    store: &ClaudeGenerationStore,
    mutation: &super::super::mutation::GenerationMutation<'_>,
    lease: &crate::namespace_lease::NamespaceLease,
    binding_path: &Path,
    transition: PendingBindingTransition,
) -> Result<()> {
    let set = store.load_set(lease)?.context(AUTHORITY_MISSING)?;
    ensure!(
        set.pending_binding_transition.as_ref() == Some(&transition),
        AUTHORITY_PENDING_MISMATCH
    );
    let target = set.active()?.clone();
    ensure!(
        transition.matches_target(&target, set.publication_generation),
        AUTHORITY_PENDING_MISMATCH
    );
    let mut next = set.clone();
    next.pending_binding_transition = None;
    if transition.target_state != AgentState::Unknown {
        let mut committed = target.clone();
        committed.binding_revision = Some(committed.revision);
        next.replace(&target, committed)?;
        next.binding_publication = Some(super::super::authority_set::ActiveBindingPublication {
            invocation_id: target.invocation_id,
            publication_generation: next.publication_generation,
            authority_revision: target.revision,
        });
    }
    store.exchange_set_in(mutation, lease, &set, &next, || {
        ensure!(
            lease.read_json::<AgentBinding>(binding_path)? == *transition.target_binding(),
            FINAL_PUBLICATION_MISMATCH
        );
        transition.verify_transcript()
    })
}
