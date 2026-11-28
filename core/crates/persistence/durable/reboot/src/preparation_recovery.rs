use super::constants::*;
use super::guard::SessionDurableMutationGuard;
use super::preparation_authority::LoadedPreparationAuthority;
use super::preparation_schema::PreparationAuthority;
use crate::paths::Paths;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
use crate::paths::identity::DurableFileVersion;
use anyhow::Result;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
use anyhow::ensure;

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn recover_existing(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
) -> Result<Option<LoadedPreparationAuthority>> {
    let Some(loaded) = super::preparation_authority::load_optional(paths, guard)? else {
        return Ok(None);
    };
    match loaded.value {
        PreparationAuthority::Retired(_) => Ok(Some(loaded)),
        PreparationAuthority::Active(_) => recover_loaded(paths, guard, &loaded).map(Some),
        PreparationAuthority::Settled(_) => recover_settled(paths, guard, &loaded).map(Some),
    }
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn recover_expected_orphan(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    expected: DurableFileVersion,
) -> Result<bool> {
    let Some(loaded) = super::preparation_authority::load_optional(paths, guard)? else {
        anyhow::bail!(PREPARATION_CHANGED);
    };
    ensure!(loaded.version == expected, PREPARATION_CHANGED);
    match loaded.value {
        PreparationAuthority::Retired(_) => Ok(false),
        PreparationAuthority::Active(_) => recover_loaded(paths, guard, &loaded).map(|_| true),
        PreparationAuthority::Settled(_) => recover_settled(paths, guard, &loaded).map(|_| true),
    }
}

pub(super) fn recover_orphan_if_present(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
) -> Result<bool> {
    let Some(loaded) = super::preparation_authority::load_optional(paths, guard)? else {
        return Ok(false);
    };
    match loaded.value {
        PreparationAuthority::Retired(_) => Ok(false),
        PreparationAuthority::Active(_) => recover_loaded(paths, guard, &loaded).map(|_| true),
        PreparationAuthority::Settled(_) => recover_settled(paths, guard, &loaded).map(|_| true),
    }
}

fn recover_settled(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedPreparationAuthority,
) -> Result<LoadedPreparationAuthority> {
    super::preparation_authority::fence_no_intent(paths, guard, loaded)?;
    super::preparation_settled::verify_installed(paths, guard, loaded)?;
    super::preparation_authority::retire(paths, guard, loaded)
}

fn recover_loaded(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedPreparationAuthority,
) -> Result<LoadedPreparationAuthority> {
    let active = super::preparation_authority::active(loaded)?.clone();
    super::preparation_authority::fence_no_intent(paths, guard, loaded)?;
    let participants = super::participant_paths::ParticipantPaths::new(paths, active.terminal_id)?;
    super::preparation_recovery_participant::recover::<
        yolo_startup_checkpoint_contract::StartupCheckpointEvidence,
    >(
        paths,
        guard,
        loaded,
        &participants.startup_checkpoint,
        active.transaction_id,
        STARTUP_CHECKPOINT_PARTICIPANT,
        &active.startup_checkpoint,
    )?;
    checkpoint()?;
    super::preparation_recovery_participant::recover::<crate::claude_custody::CustodyLedger>(
        paths,
        guard,
        loaded,
        &participants.claude_custody,
        active.transaction_id,
        CLAUDE_CUSTODY_PARTICIPANT,
        &active.claude_custody,
    )?;
    checkpoint()?;
    super::preparation_recovery_participant::recover::<
        crate::agent::claude::generation::ClaudeInvocationAuthorities,
    >(
        paths,
        guard,
        loaded,
        &participants.claude_generation,
        active.transaction_id,
        CLAUDE_GENERATION_PARTICIPANT,
        &active.claude_generation,
    )?;
    checkpoint()?;
    super::preparation_recovery_participant::recover::<crate::pty_custody::CustodyDurableRecord>(
        paths,
        guard,
        loaded,
        &participants.pty_custody,
        active.transaction_id,
        PTY_CUSTODY_PARTICIPANT,
        &active.pty_custody,
    )?;
    checkpoint()?;
    super::preparation_recovery_participant::recover::<crate::model::RuntimeRecord>(
        paths,
        guard,
        loaded,
        &participants.runtime,
        active.transaction_id,
        RUNTIME_PARTICIPANT,
        &active.runtime,
    )?;
    checkpoint()?;
    super::preparation_authority::retire(paths, guard, loaded)
}

fn checkpoint() -> Result<()> {
    #[cfg(test)]
    super::prepare_test_hook::run_after_orphan_participant()?;
    Ok(())
}
