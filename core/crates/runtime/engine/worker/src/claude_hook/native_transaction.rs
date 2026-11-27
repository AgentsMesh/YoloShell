use super::native_environment::ClaudeHookEnvironment;
use crate::agent::claude::generation::native::{
    ClaudeNativeGeneration, ClaudeNativeGenerationStore,
};
use crate::claude_hooks::ClaudeNativeRequest;
use crate::worker::WorkerState;
use anyhow::{Result, ensure};

const INVALID: &str = "Claude native callback scope or admission changed before commit";

pub(super) async fn execute(
    state: &WorkerState,
    shell: crate::process::ProcessIdentity,
    environment: &ClaudeHookEnvironment,
    peer_pid: u32,
    request: ClaudeNativeRequest,
) -> Result<ClaudeNativeGeneration> {
    let _gate = state.agent_generation_gate.lock().await;
    ensure!(
        request.is_valid()
            && request.scope.terminal_id == state.terminal_id()
            && request.scope.runtime_epoch == state.runtime_epoch(),
        INVALID
    );
    state.namespace.verify()?;
    let runtime_fence = super::native_runtime::RuntimeFence::capture(state)?;
    let store = ClaudeNativeGenerationStore::from_binding_path(
        &state.paths.agent_binding(state.terminal_id()),
    )?;
    let previous = store.load_scope_in_namespace(state.namespace.lease(), request.scope)?;
    let process_source = crate::agent::RuntimeProcessSource::new(
        state.agent_registry.process_environment_keys(),
    );
    let admitted = super::native_admission::capture(
        peer_pid,
        shell,
        request.scope,
        environment.executable(),
        &process_source,
    )?;
    let evidence = super::native_evidence::inspect(&admitted, &request, previous.as_ref())?;
    let runtime = super::binding::runtime_identity(state);
    let boot_id = crate::process::boot_id();
    let replay = previous
        .as_ref()
        .is_some_and(|record| record.replay_events.contains(&request));
    let fence = || {
        runtime_fence.verify(state)?;
        ensure!(
            super::binding::runtime_identity(state) == runtime
                && super::native_admission::capture(
                    peer_pid,
                    shell,
                    request.scope,
                    environment.executable(),
                    &process_source,
                )? == admitted,
            INVALID
        );
        if !replay {
            ensure!(
                crate::claude_hooks::native_outbox::next(environment.outbox_path(), request.scope)?
                    .as_ref()
                    == Some(&request),
                INVALID
            );
        }
        state.namespace.verify()
    };
    let record = store.apply_in_namespace(
        state.namespace.lease(),
        &request,
        runtime.worker,
        &boot_id,
        &evidence,
        fence,
    )?;
    let binding = record.recovery_binding(true);
    let previous_binding =
        crate::agent::AgentBindingStore::with_provider_runtime(
            state.paths.agent_binding(state.terminal_id()),
            std::sync::Arc::clone(&state.provider_runtime),
        )
        .read_raw_in_namespace(state.namespace.lease())?;
    store.publish_binding_in_namespace(state.namespace.lease(), &record, fence)?;
    super::binding::publish(
        state,
        super::binding::exact_commit(previous_binding.as_ref(), binding, false),
    );
    Ok(record)
}
