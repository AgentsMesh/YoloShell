use super::super::ClaudeGenerationAuthority;
use crate::agent::{AgentBinding, AgentRecoveryAvailability, AgentState};
use anyhow::{Context, Result, ensure};

pub(super) const AUTHORITY_PENDING_MISMATCH: &str =
    "Claude durable state pending publication is inconsistent";
const BINDING_TRANSCRIPT_MISSING: &str = "Claude durable state transcript is missing";

pub(super) fn binding_for_state(
    authority: &ClaudeGenerationAuthority,
    mut binding: AgentBinding,
    state: AgentState,
) -> Result<AgentBinding> {
    if state != AgentState::Unknown {
        let next = authority
            .binding(Some(&binding))
            .context(BINDING_TRANSCRIPT_MISSING)?;
        ensure!(
            authority.matches_binding_payload(&next),
            AUTHORITY_PENDING_MISMATCH
        );
        return Ok(next);
    }
    binding.state = state;
    binding.recovery_availability = AgentRecoveryAvailability::Unavailable;
    binding.updated_at_ms = crate::model::now_ms();
    Ok(binding)
}
