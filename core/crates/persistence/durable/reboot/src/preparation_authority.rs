use super::constants::*;
use super::guard::SessionDurableMutationGuard;
use super::preparation_schema::*;
use super::schema::RebootMigrationIntent;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::{DurableFileVersion, FileIdentity};
use anyhow::{Result, ensure};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub(super) struct LoadedPreparationAuthority {
    pub(super) value: PreparationAuthority,
    pub(super) identity: FileIdentity,
    pub(super) version: DurableFileVersion,
}

pub(super) fn load_optional(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
) -> Result<Option<LoadedPreparationAuthority>> {
    let path = paths.reboot_preparation_authority(guard.terminal_id());
    guard.require_preparation_authority_path(&path)?;
    if guard.namespace().file_is_missing(&path)? {
        return Ok(None);
    }
    let (value, identity, version) = guard.namespace().read_json_with_versions(&path)?;
    super::preparation_authority_validation::validate_for_guard(guard, &value)?;
    Ok(Some(LoadedPreparationAuthority {
        value,
        identity,
        version,
    }))
}

pub(super) fn pending_without_lock(
    paths: &Paths,
    namespace: &NamespaceLease,
    terminal_id: Uuid,
) -> Result<bool> {
    let path = paths.reboot_preparation_authority(terminal_id);
    if namespace.file_is_missing(&path)? {
        return Ok(false);
    }
    let (value, _): (PreparationAuthority, _) = namespace.read_json_with_identity(&path)?;
    super::preparation_authority_validation::validate_header(&value, terminal_id)?;
    let directory = namespace.directory(&paths.session_dir(terminal_id))?;
    directory.verify_path()?;
    ensure!(
        value.session_directory() == directory.durable_identity()?,
        SESSION_DIRECTORY_MISMATCH
    );
    namespace.verify()?;
    Ok(matches!(
        value,
        PreparationAuthority::Active(_) | PreparationAuthority::Settled(_)
    ))
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn publish_active(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    active: ActivePreparationAuthority,
    previous: Option<&LoadedPreparationAuthority>,
) -> Result<LoadedPreparationAuthority> {
    super::preparation_authority_validation::validate_active(guard, &active)?;
    ensure!(
        previous.is_none_or(|loaded| matches!(loaded.value, PreparationAuthority::Retired(_))),
        PREPARATION_CHANGED
    );
    if let Some(loaded) = previous {
        verify_exact(paths, guard, loaded)?;
    }
    guard.require_no_pending_intent()?;
    let value = PreparationAuthority::Active(active);
    let path = paths.reboot_preparation_authority(guard.terminal_id());
    let expected = previous.map(|loaded| loaded.identity);
    let identity =
        guard
            .namespace()
            .write_json_expected_with_commit_fence(&path, expected, &value, || {
                guard.require_no_pending_intent()?;
                previous.map_or(Ok(()), |loaded| verify_exact(paths, guard, loaded))
            })?;
    let version = super::durable_io::from_live(&guard.namespace().file(&path)?, identity)?;
    let loaded = LoadedPreparationAuthority {
        value,
        identity,
        version,
    };
    verify_exact(paths, guard, &loaded)?;
    Ok(loaded)
}

pub(super) fn retire(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedPreparationAuthority,
) -> Result<LoadedPreparationAuthority> {
    super::preparation_retirement::retire(paths, guard, loaded)
}

pub(super) fn fence_no_intent(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedPreparationAuthority,
) -> Result<()> {
    guard.require_no_pending_intent()?;
    verify_exact(paths, guard, loaded)?;
    guard.require_no_pending_intent()
}

pub(super) fn verify_exact(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    expected: &LoadedPreparationAuthority,
) -> Result<()> {
    let path = paths.reboot_preparation_authority(guard.terminal_id());
    let (value, identity): (PreparationAuthority, _) =
        guard.namespace().read_json_with_identity(&path)?;
    ensure!(
        value == expected.value && identity == expected.identity,
        PREPARATION_CHANGED
    );
    super::preparation_authority_validation::validate_for_guard(guard, &value)
}

pub(super) fn verify_intent_link(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    intent: &RebootMigrationIntent,
) -> Result<LoadedPreparationAuthority> {
    let loaded =
        load_optional(paths, guard)?.ok_or_else(|| anyhow::anyhow!(PREPARATION_CHANGED))?;
    super::preparation_authority_validation::require_intent_link(
        &loaded.value,
        loaded.version,
        intent,
    )?;
    verify_exact(paths, guard, &loaded)?;
    Ok(loaded)
}

pub(super) fn active(loaded: &LoadedPreparationAuthority) -> Result<&ActivePreparationAuthority> {
    match &loaded.value {
        PreparationAuthority::Active(value) => Ok(value),
        _ => Err(anyhow::anyhow!(PREPARATION_CHANGED)),
    }
}

pub(super) fn settled(loaded: &LoadedPreparationAuthority) -> Result<&SettledPreparationAuthority> {
    match &loaded.value {
        PreparationAuthority::Settled(value) => Ok(value),
        _ => Err(anyhow::anyhow!(PREPARATION_CHANGED)),
    }
}
