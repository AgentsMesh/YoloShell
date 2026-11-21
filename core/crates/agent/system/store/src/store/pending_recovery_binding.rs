use super::*;
use crate::agent::claude::generation::{ClaudeGenerationAuthority, ClaudeGenerationStore};
use anyhow::{Result, ensure};

pub(super) fn publish(
    store: &AgentBindingStore,
    namespace: &crate::namespace_lease::NamespaceLease,
    context: &DetectionContext,
    generations: &ClaudeGenerationStore,
    authority: &ClaudeGenerationAuthority,
    previous: Option<&AgentBinding>,
    evidence: &super::pending_recovery_evidence::RecoveryEvidence,
) -> Result<AgentBinding> {
    let authentication = authority.authentication();
    evidence.verify(authority)?;
    ensure!(
        previous.is_none()
            || previous.is_some_and(|binding| authority.binding_subject_matches(binding)),
        crate::agent::constants::recovery::AUTHORITY_CHANGED
    );
    let registry = store.registry();
    let (committed, binding) = generations.publish_binding_state_in_namespace(
        namespace,
        store.path(),
        authentication,
        authority,
        true,
        |binding| registry.validate_binding(binding, context),
    )?;
    evidence.verify(&committed)?;
    ensure!(
        committed.matches_binding(&binding),
        crate::agent::constants::recovery::AUTHORITY_CHANGED
    );
    Ok(binding)
}
