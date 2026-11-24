use crate::agent::AgentState;
use anyhow::Result;
use std::path::Path;

pub use super::generation::adversarial_test_support::{
    persist_native_durable_state, persist_native_previous_boot, persist_native_revoked,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BindingPublicationSnapshot {
    pub invocation_id: uuid::Uuid,
    pub publication_generation: u64,
    pub authority_revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationPublicationSnapshot {
    pub active_invocation_id: uuid::Uuid,
    pub active_revision: u64,
    pub active_binding_revision: Option<u64>,
    pub publication_generation: u64,
    pub binding_publication: Option<BindingPublicationSnapshot>,
    pub has_pending_binding_transition: bool,
}

pub fn publication_snapshot(binding_path: &Path) -> Result<GenerationPublicationSnapshot> {
    super::generation::adversarial_test_support::publication_snapshot(binding_path)
}

pub fn persist_durable_state(binding_path: &Path, state: AgentState) -> Result<()> {
    super::generation::adversarial_test_support::persist_durable_state(binding_path, state)
}

pub fn persist_previous_boot(
    binding_path: &Path,
    runtime_epoch: uuid::Uuid,
    current_boot_id: &str,
    previous_boot_id: &str,
) -> Result<()> {
    super::generation::adversarial_test_support::persist_previous_boot(
        binding_path,
        runtime_epoch,
        current_boot_id,
        previous_boot_id,
    )
}

pub fn rewrite_set_boot(
    set: &mut super::generation::ClaudeInvocationAuthorities,
    runtime_epoch: uuid::Uuid,
    current_boot_id: &str,
    previous_boot_id: &str,
) -> Result<()> {
    super::generation::adversarial_test_support::rewrite_set_boot(
        set,
        runtime_epoch,
        current_boot_id,
        previous_boot_id,
    )
}
