use super::constants::{PARTICIPANT_RECEIPT_CONFLICT, REBOOT_RECEIPT_SCHEMA_VERSION};
use super::guard::SessionDurableMutationGuard;
use super::receipt::{InstallReceipt, prepared_path};
use crate::paths::MAX_DURABLE_JSON_BYTES;
use crate::paths::identity::DurableFileVersion;
use anyhow::{Result, ensure};
use std::path::Path;
use uuid::Uuid;

pub(super) fn prepare(
    guard: &SessionDurableMutationGuard<'_>,
    target: &Path,
    transaction_id: Uuid,
    participant: &str,
    installed_version: DurableFileVersion,
    displaced_version: DurableFileVersion,
    cleanup: &mut super::prepare_cleanup::PrepareCleanup<'_>,
) -> Result<DurableFileVersion> {
    let receipt = InstallReceipt {
        schema_version: REBOOT_RECEIPT_SCHEMA_VERSION,
        transaction_id,
        participant: participant.to_owned(),
        installed_version,
        displaced_version,
    };
    let prepared = prepared_path(guard, target, transaction_id, participant)?;
    let cleanup_entry = prepared.reopen()?;
    let encoded = serde_json::to_vec_pretty(&receipt)?;
    ensure!(prepared.is_missing()?, PARTICIPANT_RECEIPT_CONFLICT);
    let prepared_identity = prepared.create_bytes_exclusive(&encoded, MAX_DURABLE_JSON_BYTES)?;
    cleanup.track(cleanup_entry, prepared_identity);
    prepared.verify_exact_identity(prepared_identity, MAX_DURABLE_JSON_BYTES)?;
    super::durable_io::from_live(&prepared, prepared_identity)
}
