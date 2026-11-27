use super::constants::*;
use super::guard::SessionDurableMutationGuard;
use super::schema::RebootMigrationIntent;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use anyhow::{Result, ensure};

#[derive(Clone, Debug)]
pub(crate) struct LoadedIntent {
    pub(super) value: RebootMigrationIntent,
    pub(super) identity: FileIdentity,
}

pub(super) fn load(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    terminal_id: uuid::Uuid,
) -> Result<Option<LoadedIntent>> {
    let Some((value, identity)) = guard.read_intent_optional_with_identity()? else {
        return Ok(None);
    };
    super::validation::validate(&value)?;
    ensure!(value.terminal_id == terminal_id, INTENT_INVALID);
    guard.verify_session_directory()?;
    ensure!(
        value.session_directory
            == guard
                .namespace()
                .directory(&paths.session_dir(terminal_id))?
                .durable_identity()?,
        SESSION_DIRECTORY_MISMATCH
    );
    super::preparation_authority::verify_intent_link(paths, guard, &value)?;
    Ok(Some(LoadedIntent { value, identity }))
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn publish(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    intent: &RebootMigrationIntent,
) -> Result<LoadedIntent> {
    super::validation::validate_prepared(intent)?;
    let authority = super::preparation_authority::verify_intent_link(paths, guard, intent)?;
    guard.verify_session_directory()?;
    let path = paths.reboot_migration_intent(intent.terminal_id);
    ensure!(guard.namespace().file_is_missing(&path)?, INTENT_CHANGED);
    #[cfg(test)]
    super::prepare_test_hook::run_before_intent_commit()?;
    let identity =
        guard
            .namespace()
            .write_json_expected_with_commit_fence(&path, None, intent, || {
                super::preparation_authority::fence_no_intent(paths, guard, &authority)
            })?;
    #[cfg(test)]
    super::prepare_test_hook::run_after_intent_commit()?;
    let loaded = LoadedIntent {
        value: intent.clone(),
        identity,
    };
    verify_exact(paths, guard, &loaded)?;
    Ok(loaded)
}

pub(super) fn advance(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: LoadedIntent,
    phase: super::schema::RebootMigrationPhase,
) -> Result<LoadedIntent> {
    ensure!(
        loaded.value.phase.successor() == Some(phase),
        INTENT_CHANGED
    );
    let mut replacement = loaded.value.clone();
    replacement.phase = phase;
    let path = paths.reboot_migration_intent(replacement.terminal_id);
    let identity = guard.namespace().write_json_expected_with_commit_fence(
        &path,
        Some(loaded.identity),
        &replacement,
        || verify_exact(paths, guard, &loaded),
    )?;
    let loaded = LoadedIntent {
        value: replacement,
        identity,
    };
    verify_exact(paths, guard, &loaded)?;
    Ok(loaded)
}

pub(super) fn verify_exact(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    expected: &LoadedIntent,
) -> Result<()> {
    let path = paths.reboot_migration_intent(expected.value.terminal_id);
    let (current, identity): (RebootMigrationIntent, _) =
        guard.namespace().read_json_with_identity(&path)?;
    ensure!(
        current == expected.value && identity == expected.identity,
        INTENT_CHANGED
    );
    super::preparation_authority::verify_intent_link(paths, guard, &expected.value)?;
    guard.verify_session_directory()
}

pub(super) fn clear(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedIntent,
) -> Result<()> {
    let authority = super::preparation_authority::verify_intent_link(paths, guard, &loaded.value)?;
    super::preparation_settled::verify_installed(paths, guard, &authority)?;
    let path = paths.reboot_migration_intent(loaded.value.terminal_id);
    let (current, identity): (RebootMigrationIntent, _) =
        guard.namespace().read_json_with_identity(&path)?;
    ensure!(
        current == loaded.value && identity == loaded.identity,
        INTENT_CHANGED
    );
    guard.verify_session_directory()?;
    #[cfg(test)]
    super::prepare_test_hook::run_before_intent_clear()?;
    guard
        .namespace()
        .file(&path)?
        .remove_if_unchanged_with_fence(loaded.identity, || {
            verify_exact(paths, guard, loaded)?;
            super::preparation_authority::verify_exact(paths, guard, &authority)?;
            super::preparation_settled::verify_installed(paths, guard, &authority)
        })?;
    ensure!(guard.namespace().file_is_missing(&path)?, INTENT_CHANGED);
    super::preparation_authority::verify_exact(paths, guard, &authority)?;
    super::preparation_settled::verify_installed(paths, guard, &authority)?;
    guard.verify_session_directory()
}
