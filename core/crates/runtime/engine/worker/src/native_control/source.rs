use super::constants::*;
use crate::agent::{AgentBinding, AgentBindingStore};
use crate::model::{RecoveryState, TerminalManifest};
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use crate::worker::{WorkerState, claude_hook, codex_hook};
use anyhow::{Context, Result, ensure};
use yolo_agent_control_contract::NativeControlWitness;

/// Outer caller holds the Agent generation gate, controller mutation permit and
/// exact durable runtime fence for the entire synchronous source callback.
pub(super) fn with_source<T>(
    state: &WorkerState,
    guard: &SessionDurableMutationGuard<'_>,
    expected: Option<&NativeControlWitness>,
    operation: impl FnOnce(&AgentBinding, &NativeControlWitness, &dyn Fn() -> Result<()>) -> Result<T>,
) -> Result<T> {
    let id = state.terminal_id();
    let path = state.paths.manifest(id);
    let (manifest, identity): (TerminalManifest, _) = guard
        .read_optional_with_identity(&path)?
        .context(SOURCE_CHANGED)?;
    let manifest_digest = yolo_agent_control_contract::authority_digest(&manifest)?;
    ensure!(
        !manifest.archived
            && manifest.terminal_id == id
            && manifest.recovery_state != RecoveryState::Uncertain,
        SOURCE_CHANGED
    );
    let binding = AgentBindingStore::with_provider_runtime(
        state.paths.agent_binding(id),
        std::sync::Arc::clone(&state.provider_runtime),
    )
        .read_raw_in_namespace(guard.namespace())?
        .context(SOURCE_CHANGED)?;
    let checked = |binding: &AgentBinding,
                   witness: &NativeControlWitness,
                   provider: &dyn Fn() -> Result<()>| {
        let verify = || -> Result<()> {
            provider()?;
            guard.require_no_pending_intent()?;
            let current: (TerminalManifest, _) = guard
                .read_optional_with_identity(&path)?
                .context(SOURCE_CHANGED)?;
            ensure!(
                yolo_agent_control_contract::authority_digest(&current.0)? == manifest_digest
                    && current.1 == identity,
                SOURCE_CHANGED
            );
            guard.namespace().verify()
        };
        verify()?;
        operation(binding, witness, &verify)
    };
    let provider = state
        .provider_runtime
        .get(&binding.kind)
        .context(UNAVAILABLE)?;
    match provider.descriptor().authority_kind() {
        yolo_agent_providers::AuthorityKind::ClaudeGeneration => {
            claude_hook::with_control_source(state, guard, expected, checked)
        }
        yolo_agent_providers::AuthorityKind::CodexOccurrence => {
            codex_hook::with_control_source(state, guard, expected, checked)
        }
        yolo_agent_providers::AuthorityKind::GenericHookGeneration => {
            super::generic_hook_source::with_control_source(state, guard, expected, checked)
        }
        yolo_agent_providers::AuthorityKind::None => anyhow::bail!(UNAVAILABLE),
    }
}
