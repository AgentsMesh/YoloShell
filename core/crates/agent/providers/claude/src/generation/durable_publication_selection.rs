use super::super::ClaudeGenerationAuthority;
use crate::agent::{AgentBinding, AgentState};

pub(super) fn was_published(
    publication_exists: bool,
    current: &ClaudeGenerationAuthority,
    existing: Option<&AgentBinding>,
) -> bool {
    publication_exists
        && current.binding_revision == Some(current.revision)
        && existing.is_some_and(|binding| current.matches_binding(binding))
}

pub(super) fn target(
    current: &ClaudeGenerationAuthority,
    existing: Option<&AgentBinding>,
) -> Option<AgentBinding> {
    existing
        .filter(|binding| {
            current.matches_binding_payload(binding)
                || (current.state == AgentState::Unknown
                    && current.matches_unknown_binding(binding))
        })
        .cloned()
        .or_else(|| current.transition_binding(existing))
}
