use super::constants::*;
use super::durable_io::DurableRead;
use super::guard::SessionDurableMutationGuard;
use super::schema::{RebootMigrationIntent, RequiredParticipant};
use crate::paths::identity::DurableFileVersion;
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

pub(super) fn publish<T, F>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    participant: &str,
    required: &RequiredParticipant<T>,
    prepared_identity: DurableFileVersion,
    commit_fence: &mut F,
) -> Result<()>
where
    T: DeserializeOwned + Eq + Serialize,
    F: FnMut() -> Result<()>,
{
    super::receipt::publish(
        guard,
        path,
        intent.transaction_id,
        participant,
        prepared_identity,
        || {
            commit_fence()?;
            verify_state(
                guard,
                path,
                intent,
                participant,
                required,
                prepared_identity,
            )
        },
    )?;
    Ok(())
}

fn verify_state<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    participant: &str,
    required: &RequiredParticipant<T>,
    prepared_identity: DurableFileVersion,
) -> Result<()>
where
    T: DeserializeOwned + Eq + Serialize,
{
    guard.verify_session_directory()?;
    let (current, identity): (T, _) = guard.namespace().read_json_with_durable_version(path)?;
    let entry = guard.namespace().file(path)?;
    super::publication_marker::verify_entry(&entry, identity, intent.transaction_id)?;
    ensure!(
        identity.same_content_version(required.target_version)
            && current == required.target_payload
            && crate::sha256_digest::Sha256Digest::capture(&current)? == required.target_sha256,
        PARTICIPANT_CONFLICT
    );
    super::staging_displaced::verify::<T>(
        guard,
        path,
        intent.transaction_id,
        participant,
        required.source_version,
        required.source_sha256,
    )?;
    let (receipt, identity) =
        super::receipt::read_prepared(guard, path, intent.transaction_id, participant)?
            .ok_or_else(|| anyhow::anyhow!(PARTICIPANT_RECEIPT_CONFLICT))?;
    ensure!(
        identity == prepared_identity
            && receipt.installed_version == required.target_version
            && receipt.displaced_version == required.source_version,
        PARTICIPANT_RECEIPT_CONFLICT
    );
    guard.verify_session_directory()
}
