use super::constants::*;
use super::durable_io::DurableRead;
use super::guard::SessionDurableMutationGuard;
use super::intent_io::LoadedIntent;
use super::preparation_authority::LoadedPreparationAuthority;
use super::preparation_schema::*;
use super::schema::{OptionalParticipant, RequiredParticipant};
use crate::paths::Paths;
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

pub(super) fn mark(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    intent: &LoadedIntent,
) -> Result<LoadedPreparationAuthority> {
    let current = super::preparation_authority::verify_intent_link(paths, guard, &intent.value)?;
    if matches!(current.value, PreparationAuthority::Settled(_)) {
        return Ok(current);
    }
    let settled_value = SettledPreparationAuthority {
        schema_version: REBOOT_PREPARATION_SCHEMA_VERSION,
        transaction_id: intent.value.transaction_id,
        terminal_id: intent.value.terminal_id,
        session_directory: intent.value.session_directory,
        active_version: intent.value.preparation_authority_version,
        startup_checkpoint: from_optional(&intent.value.startup_checkpoint),
        claude_custody: from_optional(&intent.value.claude_custody),
        claude_generation: from_optional(&intent.value.claude_generation),
        pty_custody: from_optional(&intent.value.pty_custody),
        runtime: from_required(&intent.value.runtime),
    };
    verify_state(paths, guard, &settled_value)?;
    let value = PreparationAuthority::Settled(settled_value);
    let path = paths.reboot_preparation_authority(intent.value.terminal_id);
    #[cfg(test)]
    super::prepare_test_hook::run_before_settled_publish()?;
    let identity = guard.namespace().write_json_expected_with_commit_fence(
        &path,
        Some(current.identity),
        &value,
        || {
            super::intent_io::verify_exact(paths, guard, intent)?;
            super::preparation_authority::verify_exact(paths, guard, &current)?;
            let PreparationAuthority::Settled(value) = &value else {
                unreachable!();
            };
            verify_state(paths, guard, value)
        },
    )?;
    let version = super::durable_io::from_live(&guard.namespace().file(&path)?, identity)?;
    let settled = LoadedPreparationAuthority {
        value,
        identity,
        version,
    };
    super::preparation_authority::verify_exact(paths, guard, &settled)?;
    verify_installed(paths, guard, &settled)?;
    super::preparation_authority_validation::require_intent_link(
        &settled.value,
        settled.version,
        &intent.value,
    )?;
    Ok(settled)
}

pub(super) fn verify_installed(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedPreparationAuthority,
) -> Result<()> {
    let settled = super::preparation_authority::settled(loaded)?;
    verify_state(paths, guard, settled)?;
    super::preparation_authority::verify_exact(paths, guard, loaded)
}

pub(super) fn verify_state(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    settled: &SettledPreparationAuthority,
) -> Result<()> {
    let participant_paths =
        super::participant_paths::ParticipantPaths::new(paths, settled.terminal_id)?;
    verify_participant::<yolo_startup_checkpoint_contract::StartupCheckpointEvidence>(
        guard,
        &participant_paths.startup_checkpoint,
        settled.transaction_id,
        STARTUP_CHECKPOINT_PARTICIPANT,
        &settled.startup_checkpoint,
    )?;
    verify_participant::<crate::claude_custody::CustodyLedger>(
        guard,
        &participant_paths.claude_custody,
        settled.transaction_id,
        CLAUDE_CUSTODY_PARTICIPANT,
        &settled.claude_custody,
    )?;
    verify_participant::<crate::agent::claude::generation::ClaudeInvocationAuthorities>(
        guard,
        &participant_paths.claude_generation,
        settled.transaction_id,
        CLAUDE_GENERATION_PARTICIPANT,
        &settled.claude_generation,
    )?;
    verify_participant::<crate::pty_custody::CustodyDurableRecord>(
        guard,
        &participant_paths.pty_custody,
        settled.transaction_id,
        PTY_CUSTODY_PARTICIPANT,
        &settled.pty_custody,
    )?;
    verify_participant::<crate::model::RuntimeRecord>(
        guard,
        &participant_paths.runtime,
        settled.transaction_id,
        RUNTIME_PARTICIPANT,
        &settled.runtime,
    )?;
    Ok(())
}

fn verify_participant<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: uuid::Uuid,
    name: &str,
    participant: &SettledParticipant,
) -> Result<()>
where
    T: DeserializeOwned + Serialize,
{
    require_helpers_missing(guard, path, transaction_id, name)?;
    match participant {
        SettledParticipant::Absent => {
            ensure!(
                guard.namespace().file_is_missing(path)?,
                PREPARATION_SOURCE_CHANGED
            );
        }
        SettledParticipant::Installed { version, sha256 } => {
            let (value, current): (T, _) =
                guard.namespace().read_json_with_durable_version(path)?;
            let entry = guard.namespace().file(path)?;
            super::publication_marker::verify_entry(&entry, current, transaction_id)?;
            ensure!(
                current.same_content_version(*version)
                    && crate::sha256_digest::Sha256Digest::capture(&value)? == *sha256,
                PREPARATION_SOURCE_CHANGED
            );
        }
    }
    Ok(())
}

fn require_helpers_missing(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: uuid::Uuid,
    name: &str,
) -> Result<()> {
    let entries = [
        super::staging::entry_for_cleanup(guard, path, transaction_id, name)?,
        super::receipt::prepared_path(guard, path, transaction_id, name)?,
        super::receipt::path(guard, path, transaction_id, name)?,
    ];
    super::preparation_recovery_io::require_all_missing(&entries)
}

fn from_optional<T>(value: &OptionalParticipant<T>) -> SettledParticipant {
    match value {
        OptionalParticipant::Absent => SettledParticipant::Absent,
        OptionalParticipant::Replace(required) => from_required(required),
    }
}

fn from_required<T>(value: &RequiredParticipant<T>) -> SettledParticipant {
    SettledParticipant::Installed {
        version: value.target_version,
        sha256: value.target_sha256,
    }
}
