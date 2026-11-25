use super::{NATIVE_PROCESS, ObservedProcess};
use anyhow::{Result, ensure};

/// Codex command hooks create their own Unix process group so timeout cleanup
/// can terminate the callback tree. The actual provider must remain foreground.
/// See rust-v0.153.4: codex-rs/hooks/src/engine/command_runner.rs, run_command.
pub(super) fn provider_boundary(
    provider: &ObservedProcess,
    callback: &ObservedProcess,
    prefix: &[ObservedProcess],
) -> Result<()> {
    ensure!(provider.is_foreground(), NATIVE_PROCESS);
    if callback.process_group != provider.process_group {
        // The direct callback child of this provider must own the new group.
        // This rejects callbacks joined to an unrelated background group.
        ensure!(
            prefix.last().is_some_and(|leader| {
                i32::try_from(leader.pid).ok() == Some(callback.process_group)
                    && leader.parent_pid == provider.pid
            }),
            NATIVE_PROCESS
        );
    }
    Ok(())
}
