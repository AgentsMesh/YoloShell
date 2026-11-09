use super::constants::REBOOT_PREPARATION_SCHEMA_VERSION;
use super::guard::SessionDurableMutationGuard;
use super::preparation_capture::CapturedParticipant;
use super::preparation_schema::ActivePreparationAuthority;
use anyhow::Result;
use uuid::Uuid;

pub(super) struct CapturedTargets {
    pub(super) source_runtime: crate::model::RuntimeRecord,
    pub(super) startup_checkpoint:
        CapturedParticipant<yolo_startup_checkpoint_contract::StartupCheckpointEvidence>,
    pub(super) claude_custody: CapturedParticipant<crate::claude_custody::CustodyLedger>,
    pub(super) claude_generation:
        CapturedParticipant<crate::agent::claude::generation::ClaudeInvocationAuthorities>,
    pub(super) pty_custody: CapturedParticipant<crate::pty_custody::CustodyDurableRecord>,
    pub(super) runtime: CapturedParticipant<crate::model::RuntimeRecord>,
}

pub(super) fn capture_targets(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &super::participant_paths::ParticipantPaths,
    targets: super::prepare::ParticipantTargets,
) -> Result<CapturedTargets> {
    let runtime = super::preparation_capture::required(guard, &paths.runtime, targets.runtime)?;
    let CapturedParticipant::Replace { source, .. } = &runtime else {
        unreachable!();
    };
    Ok(CapturedTargets {
        source_runtime: source.clone(),
        startup_checkpoint: super::preparation_capture::required(
            guard,
            &paths.startup_checkpoint,
            targets.startup_checkpoint,
        )?,
        claude_custody: super::preparation_capture::optional(
            guard,
            &paths.claude_custody,
            targets.claude_custody,
        )?,
        claude_generation: super::preparation_capture::optional(
            guard,
            &paths.claude_generation,
            targets.claude_generation,
        )?,
        pty_custody: super::preparation_capture::optional(
            guard,
            &paths.pty_custody,
            targets.pty_custody,
        )?,
        runtime,
    })
}

pub(super) fn active_authority(
    guard: &SessionDurableMutationGuard<'_>,
    transaction_id: Uuid,
    captured: &CapturedTargets,
) -> Result<ActivePreparationAuthority> {
    Ok(ActivePreparationAuthority {
        schema_version: REBOOT_PREPARATION_SCHEMA_VERSION,
        transaction_id,
        terminal_id: guard.terminal_id(),
        session_directory: guard.session_durable_identity()?,
        startup_checkpoint: captured.startup_checkpoint.plan()?,
        claude_custody: captured.claude_custody.plan()?,
        claude_generation: captured.claude_generation.plan()?,
        pty_custody: captured.pty_custody.plan()?,
        runtime: captured.runtime.plan()?,
    })
}
