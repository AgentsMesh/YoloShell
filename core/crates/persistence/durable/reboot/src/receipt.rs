use super::constants::*;
use super::durable_io::DurableRead;
use super::guard::SessionDurableMutationGuard;
use crate::paths::MAX_DURABLE_JSON_BYTES;
use crate::paths::identity::DurableFileVersion;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InstallReceipt {
    pub(super) schema_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(super) transaction_id: Uuid,
    pub(super) participant: String,
    pub(super) installed_version: DurableFileVersion,
    pub(super) displaced_version: DurableFileVersion,
}
pub(super) fn prepared_path(
    guard: &SessionDurableMutationGuard<'_>,
    target: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<PrivateFileEntry> {
    sibling_path(
        guard,
        target,
        transaction_id,
        participant,
        REBOOT_RECEIPT_PREPARED_PREFIX,
    )
}
pub(super) fn path(
    guard: &SessionDurableMutationGuard<'_>,
    target: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<PrivateFileEntry> {
    sibling_path(
        guard,
        target,
        transaction_id,
        participant,
        REBOOT_RECEIPT_INSTALLED_PREFIX,
    )
}
fn sibling_path(
    guard: &SessionDurableMutationGuard<'_>,
    target: &Path,
    transaction_id: Uuid,
    participant: &str,
    suffix: &str,
) -> Result<PrivateFileEntry> {
    guard.require_managed_path(target)?;
    ensure!(
        !transaction_id.is_nil() && !participant.is_empty(),
        PARTICIPANT_RECEIPT_CONFLICT
    );
    let target = guard.namespace().file(target)?;
    target.sibling_with_suffix(&format!("{suffix}{transaction_id}-{participant}"))
}
pub(super) fn publish<F>(
    guard: &SessionDurableMutationGuard<'_>,
    target: &Path,
    transaction_id: Uuid,
    participant: &str,
    expected: DurableFileVersion,
    mut commit_fence: F,
) -> Result<DurableFileVersion>
where
    F: FnMut() -> Result<()>,
{
    let entry = path(guard, target, transaction_id, participant)?;
    let prepared = prepared_path(guard, target, transaction_id, participant)?;
    let expected_live = super::durable_io::strict(&prepared, expected)?;
    let identity = prepared.move_to_sibling_exact_with_fence(
        &entry,
        expected_live,
        MAX_DURABLE_JSON_BYTES,
        || {
            commit_fence()?;
            guard.verify_session_directory()
        },
    )?;
    let (receipt, current, version): (InstallReceipt, _, _) =
        guard.namespace().read_json_with_versions(entry.path())?;
    ensure!(
        current == identity
            && version.same_content_version(expected)
            && receipt.transaction_id == transaction_id
            && receipt.participant == participant,
        PARTICIPANT_RECEIPT_CONFLICT
    );
    Ok(version)
}
pub(super) fn read(
    guard: &SessionDurableMutationGuard<'_>,
    target: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<Option<(InstallReceipt, DurableFileVersion)>> {
    let entry = path(guard, target, transaction_id, participant)?;
    read_at(guard, &entry, transaction_id, participant)
}
pub(super) fn read_prepared(
    guard: &SessionDurableMutationGuard<'_>,
    target: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<Option<(InstallReceipt, DurableFileVersion)>> {
    let entry = prepared_path(guard, target, transaction_id, participant)?;
    read_at(guard, &entry, transaction_id, participant)
}
pub(super) fn clear(
    guard: &SessionDurableMutationGuard<'_>,
    target: &Path,
    transaction_id: Uuid,
    participant: &str,
    expected: DurableFileVersion,
    mut commit_fence: impl FnMut() -> Result<()>,
) -> Result<()> {
    let entry = path(guard, target, transaction_id, participant)?;
    if let Some((receipt, identity)) = read(guard, target, transaction_id, participant)? {
        ensure!(
            identity.same_content_version(expected)
                && receipt.transaction_id == transaction_id
                && receipt.participant == participant,
            PARTICIPANT_RECEIPT_CONFLICT
        );
        commit_fence()?;
        let identity = super::durable_io::strict(&entry, identity)?;
        ensure!(
            entry.remove_if_exact(identity, MAX_DURABLE_JSON_BYTES)?,
            PARTICIPANT_RECEIPT_CONFLICT
        );
    }
    ensure!(entry.is_missing()?, PARTICIPANT_RECEIPT_CONFLICT);
    guard.verify_session_directory()?;
    Ok(())
}
pub(super) fn all_missing(
    guard: &SessionDurableMutationGuard<'_>,
    target: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<bool> {
    Ok(
        path(guard, target, transaction_id, participant)?.is_missing()?
            && prepared_path(guard, target, transaction_id, participant)?.is_missing()?,
    )
}
fn read_at(
    guard: &SessionDurableMutationGuard<'_>,
    entry: &PrivateFileEntry,
    transaction_id: Uuid,
    participant: &str,
) -> Result<Option<(InstallReceipt, DurableFileVersion)>> {
    if entry.is_missing()? {
        return Ok(None);
    }
    let loaded: (InstallReceipt, DurableFileVersion) = guard
        .namespace()
        .read_json_with_durable_version(entry.path())?;
    ensure!(
        loaded.0.schema_version == REBOOT_RECEIPT_SCHEMA_VERSION
            && loaded.0.transaction_id == transaction_id
            && loaded.0.participant == participant,
        PARTICIPANT_RECEIPT_CONFLICT
    );
    Ok(Some(loaded))
}
