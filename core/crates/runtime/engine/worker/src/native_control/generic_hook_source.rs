use super::constants::SOURCE_CHANGED;
use crate::agent::{AgentBinding, AgentState};
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use crate::worker::{WorkerState, lock_recover};
use anyhow::{Context, Result, ensure};
use yolo_agent_control_contract::{NativeControlWitness, NativeControlWitnessFields, authority_digest, binding_digest};
use yolo_agent_hook_generation::HookGenerationStore;

const INVALID: &str = "generic native control source is absent, stale, or revoked";

/// Caller retains the Worker generation gate, mutation permit, and durable guard.
/// All authority validation and the effect callback are synchronous under them.
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
    let path = state.paths.agent_binding(runtime.terminal_id);
    let store = HookGenerationStore::from_binding_path(&path)?;
    let (record, identity) = store
        .load_with_identity_in_namespace(durable.namespace())?
        .context(INVALID)?;
    ensure!(
        record.matches_runtime(crate::worker::claude_hook::binding::runtime_identity(state))
            && record.scope.terminal_id == runtime.terminal_id
            && record.boot_id == runtime.boot_id
            && !record.revoked
            && !record.final_session_end
            && matches!(
                record.state,
                AgentState::Running
                    | AgentState::WaitingForInput
                    | AgentState::WaitingForPermission
            )
            && record.verify_evidence()?,
        INVALID
    );
    let (binding, binding_identity): (AgentBinding, _) = durable.read_optional_with_identity(&path)?.context(INVALID)?;
    ensure!(binding == record.recovery_binding(true), INVALID);
    ensure!(binding.terminal_id == record.scope.terminal_id, INVALID);
    let process = record.scope.agent;
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
        kind: binding.kind.clone(),
        process_pid: process.pid,
        process_start_time: process.start_time,
        binding_digest: binding_digest(&binding)?,
        authority_digest: authority_digest(&(&record, identity, binding_identity))?,
    })?;
    ensure!(expected.is_none_or(|expected| expected == &witness), INVALID);
    let verify = || -> Result<()> {
        runtime_fence.verify(state)?;
        durable.require_no_pending_intent()?;
        durable.namespace().verify()?;
        ensure!(runtime.boot_id == crate::process::boot_id(), SOURCE_CHANGED);
        let (current, current_identity) = store
            .load_with_identity_in_namespace(durable.namespace())?
            .context(SOURCE_CHANGED)?;
        ensure!(
            current == record && current_identity == identity && current.verify_evidence()?,
            SOURCE_CHANGED
        );
        let (current_binding, current_binding_identity): (AgentBinding, _) = durable
            .read_optional_with_identity(&path)?
            .context(SOURCE_CHANGED)?;
        ensure!(
            current_binding == binding && current_binding_identity == binding_identity,
            SOURCE_CHANGED
        );
        ensure!(
            crate::process::process_identity_relation(process.pid, process.start_time)
                == crate::process::ProcessIdentityRelation::Matching,
            SOURCE_CHANGED
        );
        durable.require_no_pending_intent()?;
        durable.namespace().verify()
    };
    verify()?;
    operation(&binding, &witness, &verify)
}
