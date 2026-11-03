use crate::agent::AgentState;
use crate::codex::authority::{CodexBindingAuthority, CodexBindingAuthorityStore};
use anyhow::Result;
use std::path::Path;

#[path = "authority/native_fixture.rs"]
mod native;
pub use native::{
    persist_native_durable_state, persist_native_previous_boot, persist_native_revoked,
};

pub fn persist_durable_state(binding_path: &Path, state: AgentState) -> Result<()> {
    let store = CodexBindingAuthorityStore::from_binding_path(binding_path)?;
    let current: CodexBindingAuthority = crate::paths::read_json(store.path())?;
    let authority = if state == AgentState::Unknown {
        current.unavailable()?
    } else {
        current.with_adversarial_active_state(state)?
    };
    crate::paths::atomic_write_json(store.path(), &authority)?;
    crate::paths::atomic_write_json(binding_path, &authority.binding)
}
