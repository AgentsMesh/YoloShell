use super::constants::*;
use super::guard::SessionDurableMutationGuard;
use super::intent_io::LoadedIntent;
use super::schema::{OptionalParticipant, RequiredParticipant};
use crate::paths::Paths;
use anyhow::Result;
use std::path::Path;

pub(super) fn cleanup_installed(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedIntent,
) -> Result<()> {
    super::intent_io::verify_exact(paths, guard, loaded)?;
    let intent = &loaded.value;
    let participants = super::participant_paths::ParticipantPaths::new(paths, intent.terminal_id)?;
    cleanup_optional(
        guard,
        paths,
        loaded,
        &participants.startup_checkpoint,
        STARTUP_CHECKPOINT_PARTICIPANT,
        &intent.startup_checkpoint,
    )?;
    cleanup_optional(
        guard,
        paths,
        loaded,
        &participants.claude_custody,
        CLAUDE_CUSTODY_PARTICIPANT,
        &intent.claude_custody,
    )?;
    cleanup_optional(
        guard,
        paths,
        loaded,
        &participants.claude_generation,
        CLAUDE_GENERATION_PARTICIPANT,
        &intent.claude_generation,
    )?;
    cleanup_optional(
        guard,
        paths,
        loaded,
        &participants.pty_custody,
        PTY_CUSTODY_PARTICIPANT,
        &intent.pty_custody,
    )?;
    cleanup(
        guard,
        paths,
        loaded,
        &participants.runtime,
        RUNTIME_PARTICIPANT,
        &intent.runtime,
    )
}

fn cleanup_optional<T>(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    loaded: &LoadedIntent,
    path: &Path,
    participant: &str,
    value: &OptionalParticipant<T>,
) -> Result<()> {
    if let OptionalParticipant::Replace(required) = value {
        cleanup(guard, paths, loaded, path, participant, required)?;
    }
    Ok(())
}

fn cleanup<T>(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    loaded: &LoadedIntent,
    path: &Path,
    participant: &str,
    required: &RequiredParticipant<T>,
) -> Result<()> {
    let intent = &loaded.value;
    super::intent_io::verify_exact(paths, guard, loaded)?;
    super::staging::cleanup(
        guard,
        path,
        intent.transaction_id,
        participant,
        required.source_version,
    )?;
    super::intent_io::verify_exact(paths, guard, loaded)?;
    super::receipt::clear(
        guard,
        path,
        intent.transaction_id,
        participant,
        required.prepared_receipt_version,
        || super::intent_io::verify_exact(paths, guard, loaded),
    )?;
    super::intent_io::verify_exact(paths, guard, loaded)
}
