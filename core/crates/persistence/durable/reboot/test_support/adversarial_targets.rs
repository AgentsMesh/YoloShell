use super::{ParticipantTargets, SessionDurableMutationGuard};
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use anyhow::{Context, Result, ensure};

const PARTICIPANT_CHANGED: &str = "reboot test participant changed";
const CHECKPOINT_MISSING: &str = "reboot migration startup checkpoint is missing";

pub(super) fn capture(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    runtime: &RuntimeRecord,
    target: &str,
) -> Result<ParticipantTargets> {
    let mut target_runtime = runtime.clone();
    target_runtime.boot_id = target.to_owned();
    Ok(ParticipantTargets {
        startup_checkpoint: checkpoint(paths, guard, runtime, target)?,
        claude_custody: claude_custody(paths, guard, runtime, target)?,
        claude_generation: generation(paths, guard, runtime, target)?,
        pty_custody: pty_custody(paths, guard, runtime, target)?,
        runtime: target_runtime,
    })
}

fn checkpoint(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    runtime: &RuntimeRecord,
    target: &str,
) -> Result<yolo_startup_checkpoint_contract::StartupCheckpointEvidence> {
    let path = paths.worker_startup_checkpoint(runtime.terminal_id);
    let (mut value, _): (
        yolo_startup_checkpoint_contract::StartupCheckpointEvidence,
        _,
    ) = guard
        .read_optional_with_identity(&path)?
        .context(CHECKPOINT_MISSING)?;
    ensure!(
        value.boot_id == runtime.boot_id || value.boot_id == target,
        PARTICIPANT_CHANGED
    );
    value.boot_id = target.to_owned();
    Ok(value)
}

fn claude_custody(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    runtime: &RuntimeRecord,
    target: &str,
) -> Result<Option<crate::claude_custody::CustodyLedger>> {
    let path = paths
        .session_dir(runtime.terminal_id)
        .join(crate::claude_custody::LEDGER_FILE);
    let Some((mut ledger, _)): Option<(crate::claude_custody::CustodyLedger, _)> =
        guard.read_optional_with_identity(&path)?
    else {
        return Ok(None);
    };
    crate::claude_custody::reboot_test_hook::rewrite_ledger_boot(
        &mut ledger,
        &runtime.boot_id,
        target,
    )?;
    Ok(Some(ledger))
}

fn generation(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    runtime: &RuntimeRecord,
    target: &str,
) -> Result<Option<crate::agent::claude::generation::ClaudeInvocationAuthorities>> {
    let store = crate::agent::claude::generation::ClaudeGenerationStore::from_binding_path(
        &paths.agent_binding(runtime.terminal_id),
    )?;
    let Some((mut set, _)): Option<(
        crate::agent::claude::generation::ClaudeInvocationAuthorities,
        _,
    )> = guard.read_optional_with_identity(store.path())?
    else {
        return Ok(None);
    };
    crate::agent::claude::generation_test_hook::rewrite_set_boot(
        &mut set,
        runtime.runtime_epoch,
        &runtime.boot_id,
        target,
    )?;
    Ok(Some(set))
}

fn pty_custody(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    runtime: &RuntimeRecord,
    target: &str,
) -> Result<Option<crate::pty_custody::CustodyDurableRecord>> {
    let path = paths.pty_custody(runtime.terminal_id);
    let Some((mut custody, _)): Option<(crate::pty_custody::CustodyDurableRecord, _)> =
        guard.read_optional_with_identity(&path)?
    else {
        return Ok(None);
    };
    custody = crate::pty_custody::rewrite_boot_for_target(&custody, &runtime.boot_id, target)?;
    Ok(Some(custody))
}
