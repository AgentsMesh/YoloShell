use super::constants::PARTICIPANT_CONFLICT;
use super::durable_io::DurableRead;
use super::guard::SessionDurableMutationGuard;
use super::schema::{RebootMigrationIntent, RequiredParticipant};
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

pub(super) fn is_complete<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    participant: &str,
    required: &RequiredParticipant<T>,
    current: &T,
    identity: crate::paths::identity::DurableFileVersion,
    digest: crate::sha256_digest::Sha256Digest,
) -> Result<bool>
where
    T: Eq,
{
    if !helpers_missing(guard, path, intent, participant)? {
        return Ok(false);
    }
    let entry = guard.namespace().file(path)?;
    super::publication_marker::verify_entry(&entry, identity, intent.transaction_id)?;
    ensure!(
        identity.same_content_version(required.target_version)
            && digest == required.target_sha256
            && *current == required.target_payload,
        PARTICIPANT_CONFLICT
    );
    Ok(true)
}

pub(super) fn verify_complete<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    participant: &str,
    required: &RequiredParticipant<T>,
) -> Result<bool>
where
    T: DeserializeOwned + Eq + Serialize,
{
    let (current, identity): (T, _) = guard.namespace().read_json_with_durable_version(path)?;
    let digest = crate::sha256_digest::Sha256Digest::capture(&current)?;
    is_complete(
        guard,
        path,
        intent,
        participant,
        required,
        &current,
        identity,
        digest,
    )
}

fn helpers_missing(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    participant: &str,
) -> Result<bool> {
    Ok(
        super::staging::is_missing(guard, path, intent.transaction_id, participant)?
            && super::receipt::all_missing(guard, path, intent.transaction_id, participant)?,
    )
}
