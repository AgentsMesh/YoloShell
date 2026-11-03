//! Synchronous Claude permission request. The hook connection stays open while
//! a typed decision is resolved by the Worker control path.

use super::native_environment::ClaudeHookEnvironment;
use crate::claude_hooks::{ClaudeLifecycleEvent, ClaudeNativeRequest};
use crate::local_unix::LocalUnixStream;
use crate::worker::WorkerState;
use anyhow::Result;
use std::sync::Arc;

const INVALID: &str = "Claude permission request scope or admission changed";

pub(super) async fn handle(
    stream: &mut LocalUnixStream,
    state: Arc<WorkerState>,
    shell: crate::process::ProcessIdentity,
    environment: &ClaudeHookEnvironment,
    peer_pid: u32,
    request: &ClaudeNativeRequest,
) -> Result<Option<bool>> {
    let ClaudeLifecycleEvent::InteractionRequested {
        interaction,
        prompt,
        ..
    } = &request.event
    else {
        anyhow::bail!(INVALID);
    };
    anyhow::ensure!(
        request.is_valid()
            && request.scope.terminal_id == state.terminal_id()
            && request.scope.runtime_epoch == state.runtime_epoch(),
        INVALID
    );
    state.namespace.verify()?;
    let runtime_fence = super::native_runtime::RuntimeFence::capture(&state)?;
    let process_source = crate::agent::RuntimeProcessSource::new(
        state.agent_registry.process_environment_keys(),
    );
    super::native_admission::capture(
        peer_pid,
        shell,
        request.scope,
        environment.executable(),
        &process_source,
    )?;
    let pending = super::super::native_control::interaction::permission(
        *interaction,
        prompt.clone(),
    )?;
    let decision = state.register_interaction(
        request.scope.terminal_id,
        request.scope.runtime_epoch,
        crate::agent::hooks::ProcessIdentity::new(
            request.scope.agent.pid,
            request.scope.agent.start_time,
        ),
        pending,
    );
    let selected = tokio::select! {
        decision = decision => decision.ok(),
        _ = tokio::time::sleep(crate::claude_hooks::CLAUDE_PERMISSION_WAIT) => None,
        _ = peer_closed(stream) => None,
    };
    state.clear_interaction(*interaction);
    runtime_fence.verify(&state).ok();
    Ok(selected)
}

/// The hook sends exactly one frame and then waits. Extra bytes or EOF mean the
/// provider abandoned the request, so the pending interaction must not linger.
async fn peer_closed(stream: &mut LocalUnixStream) -> bool {
    loop {
        match stream.try_read(&mut [0u8; 1]) {
            Ok(_) => return true,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if stream.readable().await.is_err() {
                    return true;
                }
            }
            Err(_) => return true,
        }
    }
}
