use super::constants::PARTICIPANT_CONFLICT;
use super::durable_io::DurableRead;
use super::guard::SessionDurableMutationGuard;
use crate::paths::identity::DurableFileVersion;
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;
use uuid::Uuid;

pub(super) fn read<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
) -> Result<(T, DurableFileVersion)>
where
    T: DeserializeOwned,
{
    let stage = super::staging::entry_for_displaced(guard, path, transaction_id, participant)?;
    let loaded = guard
        .namespace()
        .read_json_with_durable_version(stage.path())?;
    super::durable_io::content(&stage, loaded.1)?;
    Ok(loaded)
}

pub(super) fn verify<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
    expected_identity: DurableFileVersion,
    expected_sha256: crate::sha256_digest::Sha256Digest,
) -> Result<()>
where
    T: DeserializeOwned + Serialize,
{
    let (source, identity): (T, _) = read(guard, path, transaction_id, participant)?;
    ensure!(
        identity.same_content_version(expected_identity)
            && crate::sha256_digest::Sha256Digest::capture(&source)? == expected_sha256,
        PARTICIPANT_CONFLICT
    );
    Ok(())
}
