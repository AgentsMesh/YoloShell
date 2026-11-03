use super::constants::*;
use super::guard::SessionDurableMutationGuard;
use super::preparation_authority::LoadedPreparationAuthority;
use super::preparation_schema::{PreparationAuthority, RetiredPreparationAuthority};
use crate::paths::Paths;
use anyhow::Result;

pub(super) fn retire(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedPreparationAuthority,
) -> Result<LoadedPreparationAuthority> {
    let (terminal_id, session_directory) = match &loaded.value {
        PreparationAuthority::Active(value) => (value.terminal_id, value.session_directory),
        PreparationAuthority::Settled(value) => (value.terminal_id, value.session_directory),
        PreparationAuthority::Retired(_) => anyhow::bail!(PREPARATION_CHANGED),
    };
    transition_fence(paths, guard, loaded)?;
    #[cfg(test)]
    super::prepare_test_hook::run_before_preparation_retire()?;
    let value = PreparationAuthority::Retired(RetiredPreparationAuthority {
        schema_version: REBOOT_PREPARATION_SCHEMA_VERSION,
        terminal_id,
        session_directory,
    });
    let path = paths.reboot_preparation_authority(terminal_id);
    let identity = guard.namespace().write_json_expected_with_commit_fence(
        &path,
        Some(loaded.identity),
        &value,
        || transition_fence(paths, guard, loaded),
    )?;
    let version = super::durable_io::from_live(&guard.namespace().file(&path)?, identity)?;
    let retired = LoadedPreparationAuthority {
        value,
        identity,
        version,
    };
    post_fence(paths, guard, loaded, &retired)?;
    Ok(retired)
}

fn transition_fence(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedPreparationAuthority,
) -> Result<()> {
    super::preparation_authority::verify_exact(paths, guard, loaded)?;
    state_fence(paths, guard, loaded)?;
    super::preparation_authority::verify_exact(paths, guard, loaded)
}

fn post_fence(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    previous: &LoadedPreparationAuthority,
    retired: &LoadedPreparationAuthority,
) -> Result<()> {
    super::preparation_authority::verify_exact(paths, guard, retired)?;
    state_fence(paths, guard, previous)?;
    super::preparation_authority::verify_exact(paths, guard, retired)
}

fn state_fence(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedPreparationAuthority,
) -> Result<()> {
    guard.require_no_pending_intent()?;
    match &loaded.value {
        PreparationAuthority::Active(value) => verify_active(paths, guard, value)?,
        PreparationAuthority::Settled(value) => {
            super::preparation_settled::verify_state(paths, guard, value)?
        }
        PreparationAuthority::Retired(_) => anyhow::bail!(PREPARATION_CHANGED),
    }
    guard.require_no_pending_intent()
}

fn verify_active(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    active: &super::preparation_schema::ActivePreparationAuthority,
) -> Result<()> {
    let participants = super::participant_paths::ParticipantPaths::new(paths, active.terminal_id)?;
    verify::<yolo_startup_checkpoint_contract::StartupCheckpointEvidence>(
        guard,
        &participants.startup_checkpoint,
        active,
        STARTUP_CHECKPOINT_PARTICIPANT,
        &active.startup_checkpoint,
    )?;
    verify::<crate::claude_custody::CustodyLedger>(
        guard,
        &participants.claude_custody,
        active,
        CLAUDE_CUSTODY_PARTICIPANT,
        &active.claude_custody,
    )?;
    verify::<crate::agent::claude::generation::ClaudeInvocationAuthorities>(
        guard,
        &participants.claude_generation,
        active,
        CLAUDE_GENERATION_PARTICIPANT,
        &active.claude_generation,
    )?;
    verify::<crate::pty_custody::CustodyDurableRecord>(
        guard,
        &participants.pty_custody,
        active,
        PTY_CUSTODY_PARTICIPANT,
        &active.pty_custody,
    )?;
    verify::<crate::model::RuntimeRecord>(
        guard,
        &participants.runtime,
        active,
        RUNTIME_PARTICIPANT,
        &active.runtime,
    )
}

fn verify<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    active: &super::preparation_schema::ActivePreparationAuthority,
    name: &str,
    plan: &super::preparation_schema::ParticipantPreparation,
) -> Result<()>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    super::preparation_recovery_participant::verify_quiescent::<T>(
        guard,
        path,
        active.transaction_id,
        name,
        plan,
    )
}
