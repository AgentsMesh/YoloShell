use crate::agent::codex::authority::{CodexAuthorityStatus, CodexBindingAuthority};
use crate::agent::{AgentBinding, AgentBindingStore, AgentKind};
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use crate::worker::{WorkerState, lock_recover};
use anyhow::{Context, Result, ensure};
use yolo_agent_control_contract::{
    NativeControlWitness, NativeControlWitnessFields, authority_digest, binding_digest,
};

const INVALID: &str = "Codex native control source is absent, stale, or revoked";

/// Caller retains Worker generation/mutation admission and the session durable
/// guard; source locks are additionally retained through the synchronous callback.
pub(in crate::worker) fn with_control_source<T>(
    state: &WorkerState,
    durable: &SessionDurableMutationGuard<'_>,
    expected: Option<&NativeControlWitness>,
    operation: impl FnOnce(&AgentBinding, &NativeControlWitness, &dyn Fn() -> Result<()>) -> Result<T>,
) -> Result<T> {
    durable.require_no_pending_intent()?;
    let runtime = lock_recover(&state.record).clone();
    ensure!(durable.terminal_id() == runtime.terminal_id, INVALID);
    ensure!(runtime.boot_id == crate::process::boot_id(), INVALID);
    let runtime_fence = crate::worker::claude_hook::native_runtime::RuntimeFence::capture(state)?;
    let authority_store =
        crate::agent::codex::authority::CodexBindingAuthorityStore::from_binding_path(
            &state.paths.agent_binding(runtime.terminal_id),
        )?;
    let (authority, identity): (CodexBindingAuthority, _) = durable
        .read_optional_with_identity(authority_store.path())?
        .context(INVALID)?;
    authority.validate()?;
    ensure!(
        authority.status == CodexAuthorityStatus::Active
            && authority
                .matches_runtime(crate::worker::claude_hook::binding::runtime_identity(state))
            && authority.resume_boot_id() == Some(runtime.boot_id.as_str()),
        INVALID
    );
    authority.verify_resume_evidence()?;
    let (binding, binding_identity): (AgentBinding, _) = durable
        .read_optional_with_identity(&state.paths.agent_binding(runtime.terminal_id))?
        .context(INVALID)?;
    ensure!(
        binding == authority.binding && binding.kind == AgentKind::Codex,
        INVALID
    );
    state
        .codex_hook_ingress
        .with_control_route(&authority, |entry| {
            entry
                .runtime()
                .with_control_authority(entry, &authority, state.agent_registry.as_ref(), |lineage, verify_runtime| {
                    let process = authority.provider_process();
                    ensure!(
                        crate::process::process_identity_relation(process.pid, process.start_time)
                            == crate::process::ProcessIdentityRelation::Matching,
                        INVALID
                    );
                    let witness = NativeControlWitness::new(NativeControlWitnessFields {
                        terminal_id: runtime.terminal_id,
                        runtime_epoch: runtime.runtime_epoch,
                        worker_id: runtime.worker_id,
                        boot_id: runtime.boot_id.clone(),
                        kind: AgentKind::Codex,
                        process_pid: process.pid,
                        process_start_time: process.start_time,
                        binding_digest: binding_digest(&binding)?,
                        authority_digest: authority_digest(&(
                            &authority,
                            identity,
                            binding_identity,
                            lineage,
                        ))?,
                    })?;
                    ensure!(
                        expected.is_none_or(|expected| expected == &witness),
                        INVALID
                    );
                    let verify = || -> Result<()> {
                        runtime_fence.verify(state)?;
                        durable.require_no_pending_intent()?;
                        durable.namespace().verify()?;
                        ensure!(runtime.boot_id == crate::process::boot_id(), INVALID);
                        let (current, current_identity): (CodexBindingAuthority, _) = durable
                            .read_optional_with_identity(authority_store.path())?
                            .context(INVALID)?;
                        ensure!(
                            current == authority && current_identity == identity,
                            INVALID
                        );
                        current.verify_resume_evidence()?;
                        let (current_binding, current_binding_identity): (AgentBinding, _) =
                            durable
                                .read_optional_with_identity(
                                    &state.paths.agent_binding(runtime.terminal_id),
                                )?
                                .context(INVALID)?;
                        ensure!(
                            current_binding == binding
                                && current_binding_identity == binding_identity,
                            INVALID
                        );
                        ensure!(
                            crate::process::process_identity_relation(
                                process.pid,
                                process.start_time
                            ) == crate::process::ProcessIdentityRelation::Matching,
                            INVALID
                        );
                        verify_runtime()?;
                        durable.require_no_pending_intent()?;
                        durable.namespace().verify()
                    };
                    verify()?;
                    operation(&binding, &witness, &verify)
                })
        })
}
