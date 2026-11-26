use super::constants::{PARTICIPANT_CONFLICT, REBOOT_STAGE_PREFIX};
use super::guard::SessionDurableMutationGuard;
use crate::paths::MAX_DURABLE_JSON_BYTES;
use crate::paths::identity::DurableFileVersion;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Result, ensure};
use serde::de::DeserializeOwned;
use std::path::Path;
use uuid::Uuid;

pub(super) fn verify<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
    target: &T,
    identity: DurableFileVersion,
) -> Result<()>
where
    T: DeserializeOwned + Eq,
{
    verify_payload(
        guard,
        &entry(guard, path, transaction_id, participant)?,
        transaction_id,
        target,
        identity,
    )
}

pub(super) fn install<T, F>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
    source: DurableFileVersion,
    target: DurableFileVersion,
    payload: &T,
    commit_fence: F,
) -> Result<()>
where
    T: DeserializeOwned + Eq,
    F: FnMut() -> Result<()>,
{
    let current = guard.namespace().file(path)?;
    let stage = entry(guard, path, transaction_id, participant)?;
    verify_payload(guard, &stage, transaction_id, payload, target)?;
    let source = super::durable_io::strict(&current, source)?;
    let target = super::durable_io::strict(&stage, target)?;
    current.exchange_sibling_exact(&stage, source, target, MAX_DURABLE_JSON_BYTES, commit_fence)?;
    guard.verify_session_directory()
}

pub(super) fn identities(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<(DurableFileVersion, DurableFileVersion)> {
    let current = guard.namespace().file(path)?;
    let displaced = entry(guard, path, transaction_id, participant)?;
    Ok((
        super::durable_io::capture(&current)?,
        super::durable_io::capture(&displaced)?,
    ))
}

pub(super) fn is_missing(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<bool> {
    entry(guard, path, transaction_id, participant)?.is_missing()
}

pub(super) fn cleanup(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
    source: DurableFileVersion,
) -> Result<()> {
    let stage = entry(guard, path, transaction_id, participant)?;
    if !stage.is_missing()? {
        let source = super::durable_io::content(&stage, source)?;
        ensure!(
            stage.remove_if_content_version(source, MAX_DURABLE_JSON_BYTES)?,
            PARTICIPANT_CONFLICT
        );
    }
    guard.verify_session_directory()
}

#[cfg(test)]
pub(super) fn discard_prepared(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
    target: DurableFileVersion,
) -> Result<()> {
    let stage = entry(guard, path, transaction_id, participant)?;
    if !stage.is_missing()? {
        let target = super::durable_io::strict(&stage, target)?;
        ensure!(
            stage.remove_if_exact(target, MAX_DURABLE_JSON_BYTES)?,
            PARTICIPANT_CONFLICT
        );
    }
    Ok(())
}

fn verify_payload<T>(
    guard: &SessionDurableMutationGuard<'_>,
    entry: &PrivateFileEntry,
    transaction_id: Uuid,
    expected: &T,
    identity: DurableFileVersion,
) -> Result<()>
where
    T: DeserializeOwned + Eq,
{
    super::durable_io::strict(entry, identity)?;
    super::publication_marker::verify_entry(entry, identity, transaction_id)?;
    let actual: T = guard.namespace().read_json(entry.path())?;
    ensure!(actual == *expected, PARTICIPANT_CONFLICT);
    super::durable_io::strict(entry, identity)?;
    super::publication_marker::verify_entry(entry, identity, transaction_id)
}

#[cfg(test)]
pub(super) fn entry_for_test(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<PrivateFileEntry> {
    entry(guard, path, transaction_id, participant)
}

pub(super) fn entry_for_displaced(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<PrivateFileEntry> {
    entry(guard, path, transaction_id, participant)
}

pub(super) fn entry_for_cleanup(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<PrivateFileEntry> {
    entry(guard, path, transaction_id, participant)
}

pub(super) fn entry(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<PrivateFileEntry> {
    guard.require_managed_path(path)?;
    ensure!(
        !transaction_id.is_nil() && !participant.is_empty(),
        PARTICIPANT_CONFLICT
    );
    guard.namespace().file(path)?.sibling_with_suffix(&format!(
        "{REBOOT_STAGE_PREFIX}{transaction_id}-{participant}"
    ))
}
