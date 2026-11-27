use super::constants::*;
use super::durable_io::DurableRead;
use super::guard::SessionDurableMutationGuard;
use super::preparation_authority::LoadedPreparationAuthority;
use crate::paths::MAX_DURABLE_JSON_BYTES;
use crate::paths::Paths;
use crate::paths::identity::DurableFileVersion;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub(super) fn verify_quiescent<T>(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    authority: &LoadedPreparationAuthority,
    source_path: &Path,
    source_version: DurableFileVersion,
    source_sha256: crate::sha256_digest::Sha256Digest,
    _target_sha256: crate::sha256_digest::Sha256Digest,
) -> Result<()>
where
    T: DeserializeOwned + Serialize,
{
    super::preparation_authority::fence_no_intent(paths, guard, authority)?;
    let (source, identity): (T, _) = guard
        .namespace()
        .read_json_with_durable_version(source_path)?;
    ensure!(
        identity == source_version
            && crate::sha256_digest::Sha256Digest::capture(&source)? == source_sha256,
        PREPARATION_SOURCE_CHANGED
    );
    super::preparation_authority::fence_no_intent(paths, guard, authority)
}

pub(super) fn load_stage<T>(
    guard: &SessionDurableMutationGuard<'_>,
    stage: &PrivateFileEntry,
    transaction_id: uuid::Uuid,
    target_sha256: crate::sha256_digest::Sha256Digest,
) -> Result<Option<(T, DurableFileVersion)>>
where
    T: DeserializeOwned + Serialize,
{
    if stage.is_missing()? {
        return Ok(None);
    }
    let loaded: (T, DurableFileVersion) = guard
        .namespace()
        .read_json_with_durable_version(stage.path())?;
    super::durable_io::strict(stage, loaded.1)?;
    super::publication_marker::verify_entry(stage, loaded.1, transaction_id)?;
    ensure!(
        crate::sha256_digest::Sha256Digest::capture(&loaded.0)? == target_sha256,
        PREPARATION_ARTIFACT_CHANGED
    );
    Ok(Some(loaded))
}

pub(super) fn validate_pair<T>(
    stage: Option<&(T, DurableFileVersion)>,
    receipt: Option<&(super::receipt::InstallReceipt, DurableFileVersion)>,
    source_version: DurableFileVersion,
) -> Result<()> {
    match (stage, receipt) {
        (None, None) | (Some(_), None) => Ok(()),
        (Some((_, stage_identity)), Some((receipt, _))) => {
            ensure!(
                receipt.installed_version == *stage_identity
                    && receipt.displaced_version == source_version,
                PREPARATION_ARTIFACT_CHANGED
            );
            Ok(())
        }
        (None, Some(_)) => Err(anyhow::anyhow!(PREPARATION_ARTIFACT_CHANGED)),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn remove_exact<T>(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    authority: &LoadedPreparationAuthority,
    source_path: &Path,
    source_version: DurableFileVersion,
    source_sha256: crate::sha256_digest::Sha256Digest,
    entry: &PrivateFileEntry,
    identity: DurableFileVersion,
) -> Result<()>
where
    T: DeserializeOwned + Serialize,
{
    #[cfg(test)]
    super::prepare_test_hook::run_before_orphan_mutation()?;
    fence_source::<T>(
        paths,
        guard,
        authority,
        source_path,
        source_version,
        source_sha256,
    )?;
    let identity = super::durable_io::strict(entry, identity)?;
    ensure!(
        entry.remove_if_exact(identity, MAX_DURABLE_JSON_BYTES)?,
        PREPARATION_ARTIFACT_CHANGED
    );
    fence_source::<T>(
        paths,
        guard,
        authority,
        source_path,
        source_version,
        source_sha256,
    )
}

pub(super) fn fence_source<T>(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    authority: &LoadedPreparationAuthority,
    source_path: &Path,
    source_version: DurableFileVersion,
    source_sha256: crate::sha256_digest::Sha256Digest,
) -> Result<()>
where
    T: DeserializeOwned + Serialize,
{
    super::preparation_authority::fence_no_intent(paths, guard, authority)?;
    let (source, identity): (T, _) = guard
        .namespace()
        .read_json_with_durable_version(source_path)?;
    ensure!(
        identity == source_version
            && crate::sha256_digest::Sha256Digest::capture(&source)? == source_sha256,
        PREPARATION_SOURCE_CHANGED
    );
    super::preparation_authority::fence_no_intent(paths, guard, authority)
}

pub(super) fn require_all_missing(entries: &[PrivateFileEntry]) -> Result<()> {
    for entry in entries {
        ensure!(entry.is_missing()?, PREPARATION_ARTIFACT_CHANGED);
    }
    Ok(())
}
