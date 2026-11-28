use super::constants::PARTICIPANT_CONFLICT;
use super::guard::SessionDurableMutationGuard;
use crate::paths::MAX_DURABLE_JSON_BYTES;
use crate::paths::identity::DurableFileVersion;
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;
use uuid::Uuid;

pub(super) fn prepare<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
    target: &T,
    cleanup: &mut super::prepare_cleanup::PrepareCleanup<'_>,
) -> Result<DurableFileVersion>
where
    T: DeserializeOwned + Eq + Serialize,
{
    let stage = super::staging::entry(guard, path, transaction_id, participant)?;
    let cleanup_entry = stage.reopen()?;
    ensure!(stage.is_missing()?, PARTICIPANT_CONFLICT);
    let bytes = serde_json::to_vec_pretty(target)?;
    ensure!(
        u64::try_from(bytes.len())? <= MAX_DURABLE_JSON_BYTES,
        PARTICIPANT_CONFLICT
    );
    let identity =
        stage.create_bytes_exclusive_initialized(&bytes, MAX_DURABLE_JSON_BYTES, |file| {
            super::publication_marker::install(file, transaction_id)
        })?;
    cleanup.track(cleanup_entry, identity);
    let identity = super::durable_io::from_live(&stage, identity)?;
    super::staging::verify(guard, path, transaction_id, participant, target, identity)?;
    Ok(identity)
}
