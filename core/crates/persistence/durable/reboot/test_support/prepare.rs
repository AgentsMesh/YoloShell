use super::constants::*;
use super::guard::SessionDurableMutationGuard;
use super::preparation_capture::CapturedParticipant;
use super::schema::*;
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use anyhow::{Result, ensure};
use uuid::Uuid;

pub(crate) struct ParticipantTargets {
    pub startup_checkpoint: yolo_startup_checkpoint_contract::StartupCheckpointEvidence,
    pub(crate) claude_custody: Option<crate::claude_custody::CustodyLedger>,
    pub(crate) claude_generation:
        Option<crate::agent::claude::generation::ClaudeInvocationAuthorities>,
    pub(crate) pty_custody: Option<crate::pty_custody::CustodyDurableRecord>,
    pub(crate) runtime: RuntimeRecord,
}

pub(crate) fn prepare_intent(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    targets: ParticipantTargets,
) -> Result<RebootMigrationIntent> {
    ensure!(
        targets.runtime.terminal_id == guard.terminal_id(),
        INTENT_INVALID
    );
    let previous = super::preparation_recovery::recover_existing(paths, guard)?;
    let participant_paths =
        super::participant_paths::ParticipantPaths::new(paths, guard.terminal_id())?;
    let captured = super::prepare_capture::capture_targets(guard, &participant_paths, targets)?;
    let transaction_id = Uuid::new_v4();
    let active = super::prepare_capture::active_authority(guard, transaction_id, &captured)?;
    let authority =
        super::preparation_authority::publish_active(paths, guard, active, previous.as_ref())?;
    #[cfg(test)]
    super::prepare_test_hook::run_after_authority_publish()?;
    let mut cleanup = super::prepare_cleanup::PrepareCleanup::new(paths, guard, &authority);
    let prepared = prepare_inner(
        paths,
        guard,
        participant_paths,
        captured,
        transaction_id,
        &authority,
        &mut cleanup,
    );
    match prepared {
        Ok(intent) => {
            cleanup.disarm();
            Ok(intent)
        }
        Err(error) => cleanup.fail(error),
    }
}

fn prepare_inner(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    participant_paths: super::participant_paths::ParticipantPaths,
    captured: super::prepare_capture::CapturedTargets,
    transaction_id: Uuid,
    authority: &super::preparation_authority::LoadedPreparationAuthority,
    cleanup: &mut super::prepare_cleanup::PrepareCleanup<'_>,
) -> Result<RebootMigrationIntent> {
    let startup_checkpoint = super::prepare_participant::required(
        paths,
        guard,
        authority,
        &participant_paths.startup_checkpoint,
        transaction_id,
        STARTUP_CHECKPOINT_PARTICIPANT,
        captured.startup_checkpoint,
        cleanup,
    )?;
    let claude_custody = super::prepare_participant::optional(
        paths,
        guard,
        authority,
        &participant_paths.claude_custody,
        transaction_id,
        CLAUDE_CUSTODY_PARTICIPANT,
        captured.claude_custody,
        cleanup,
    )?;
    let claude_generation = super::prepare_participant::optional(
        paths,
        guard,
        authority,
        &participant_paths.claude_generation,
        transaction_id,
        CLAUDE_GENERATION_PARTICIPANT,
        captured.claude_generation,
        cleanup,
    )?;
    let mut pty_custody = super::prepare_participant::optional(
        paths,
        guard,
        authority,
        &participant_paths.pty_custody,
        transaction_id,
        PTY_CUSTODY_PARTICIPANT,
        captured.pty_custody,
        cleanup,
    )?;
    apply_pty_rollover_proof(&startup_checkpoint, &mut pty_custody)?;
    let CapturedParticipant::Replace {
        source,
        source_version,
        target,
    } = captured.runtime
    else {
        unreachable!();
    };
    let runtime = super::prepare_participant::required_from_source(
        paths,
        guard,
        authority,
        &participant_paths.runtime,
        transaction_id,
        RUNTIME_PARTICIPANT,
        source_version,
        &source,
        &target,
        cleanup,
    )?;
    let intent = RebootMigrationIntent {
        schema_version: REBOOT_INTENT_SCHEMA_VERSION,
        transaction_id,
        terminal_id: guard.terminal_id(),
        runtime_epoch: captured.source_runtime.runtime_epoch,
        worker_id: captured.source_runtime.worker_id,
        controller_generation: captured.source_runtime.controller_generation,
        source_boot_id: captured.source_runtime.boot_id,
        target_boot_id: target.boot_id.clone(),
        session_directory: guard.session_durable_identity()?,
        preparation_authority_version: authority.version,
        phase: RebootMigrationPhase::Prepared,
        startup_checkpoint,
        claude_custody,
        claude_generation,
        pty_custody,
        runtime,
    };
    super::validation::validate_prepared(&intent)?;
    super::preparation_authority::verify_intent_link(paths, guard, &intent)?;
    Ok(intent)
}

fn apply_pty_rollover_proof(
    checkpoint: &OptionalParticipant<yolo_startup_checkpoint_contract::StartupCheckpointEvidence>,
    custody: &mut OptionalParticipant<crate::pty_custody::CustodyDurableRecord>,
) -> Result<()> {
    let OptionalParticipant::Replace(checkpoint) = checkpoint else {
        return Ok(());
    };
    let Some(rollover) = checkpoint
        .target_payload
        .pending_pty_custody_rollover
        .as_ref()
    else {
        return Ok(());
    };
    let OptionalParticipant::Replace(required) = custody else {
        anyhow::bail!(PARTICIPANT_CONFLICT);
    };
    required.source_object_sha256 = Some(rollover.source_sha256);
    Ok(())
}

pub(crate) fn publish_intent(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    intent: &RebootMigrationIntent,
) -> Result<super::intent_io::LoadedIntent> {
    super::intent_io::publish(paths, guard, intent)
}

pub(crate) fn prepare_and_publish_intent(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    targets: ParticipantTargets,
) -> Result<super::intent_io::LoadedIntent> {
    let intent = prepare_intent(paths, guard, targets)?;
    match publish_intent(paths, guard, &intent) {
        Ok(loaded) => Ok(loaded),
        Err(error) => super::prepare_cleanup::cleanup_unpublished(paths, guard, &intent, error),
    }
}
