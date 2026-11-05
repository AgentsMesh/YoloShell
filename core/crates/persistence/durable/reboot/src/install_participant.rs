use super::constants::*;
use super::durable_io::DurableRead;
use super::guard::SessionDurableMutationGuard;
use super::schema::{RebootMigrationIntent, RequiredParticipant};
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

pub(super) fn install<T, F>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    participant: &str,
    required: &RequiredParticipant<T>,
    mut commit_fence: F,
) -> Result<()>
where
    T: Clone + DeserializeOwned + Eq + Serialize,
    F: FnMut() -> Result<()>,
{
    guard.verify_session_directory()?;
    let (current, identity): (T, _) = guard.namespace().read_json_with_durable_version(path)?;
    let digest = crate::sha256_digest::Sha256Digest::capture(&current)?;
    if super::install_cleanup_state::is_complete(
        guard,
        path,
        intent,
        participant,
        required,
        &current,
        identity,
        digest,
    )? {
        return Ok(());
    }
    if let Some((receipt, receipt_identity)) =
        super::receipt::read(guard, path, intent.transaction_id, participant)?
    {
        verify_marker(guard, path, intent, required, identity)?;
        ensure!(
            receipt_identity.same_content_version(required.prepared_receipt_version)
                && receipt
                    .installed_version
                    .same_content_version(required.target_version)
                && receipt
                    .displaced_version
                    .same_content_version(required.source_version)
                && identity.same_content_version(receipt.installed_version)
                && digest == required.target_sha256
                && current == required.target_payload,
            PARTICIPANT_RECEIPT_CONFLICT
        );
        return Ok(());
    }
    let prepared = prepared_receipt(guard, path, intent, participant, required)?;
    if identity.same_content_version(required.target_version) {
        verify_marker(guard, path, intent, required, identity)?;
        ensure!(
            current == required.target_payload && digest == required.target_sha256,
            PARTICIPANT_CONFLICT
        );
    } else {
        install_source(
            guard,
            path,
            intent,
            participant,
            required,
            digest,
            identity,
            &mut commit_fence,
        )?;
    }
    let (installed, displaced) =
        super::staging::identities(guard, path, intent.transaction_id, participant)?;
    verify_marker(guard, path, intent, required, installed)?;
    ensure!(
        installed.same_content_version(required.target_version)
            && displaced.same_content_version(required.source_version),
        PARTICIPANT_CONFLICT
    );
    super::install_publication::publish(
        guard,
        path,
        intent,
        participant,
        required,
        prepared,
        &mut commit_fence,
    )?;
    verify(guard, path, intent, participant, required)
}

pub(super) fn verify<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    participant: &str,
    required: &RequiredParticipant<T>,
) -> Result<()>
where
    T: DeserializeOwned + Eq + Serialize,
{
    if super::install_cleanup_state::verify_complete(guard, path, intent, participant, required)? {
        return guard.verify_session_directory();
    }
    let (receipt, receipt_identity) =
        super::receipt::read(guard, path, intent.transaction_id, participant)?
            .ok_or_else(|| anyhow::anyhow!(PARTICIPANT_RECEIPT_CONFLICT))?;
    let (current, identity): (T, _) = guard.namespace().read_json_with_durable_version(path)?;
    verify_marker(guard, path, intent, required, identity)?;
    ensure!(
        receipt_identity.same_content_version(required.prepared_receipt_version)
            && receipt
                .installed_version
                .same_content_version(required.target_version)
            && receipt
                .displaced_version
                .same_content_version(required.source_version)
            && identity.same_content_version(receipt.installed_version)
            && current == required.target_payload
            && crate::sha256_digest::Sha256Digest::capture(&current)? == required.target_sha256,
        PARTICIPANT_CONFLICT
    );
    guard.verify_session_directory()
}

fn verify_marker<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    required: &RequiredParticipant<T>,
    identity: crate::paths::identity::DurableFileVersion,
) -> Result<()> {
    ensure!(
        identity.same_content_version(required.target_version),
        PARTICIPANT_CONFLICT
    );
    let entry = guard.namespace().file(path)?;
    super::publication_marker::verify_entry(&entry, identity, intent.transaction_id)
}

fn prepared_receipt<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    participant: &str,
    required: &RequiredParticipant<T>,
) -> Result<crate::paths::identity::DurableFileVersion> {
    let (receipt, identity) =
        super::receipt::read_prepared(guard, path, intent.transaction_id, participant)?
            .ok_or_else(|| anyhow::anyhow!(PARTICIPANT_RECEIPT_CONFLICT))?;
    ensure!(
        identity == required.prepared_receipt_version
            && receipt.installed_version == required.target_version
            && receipt.displaced_version == required.source_version,
        PARTICIPANT_RECEIPT_CONFLICT
    );
    Ok(identity)
}

fn install_source<T, F>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    participant: &str,
    required: &RequiredParticipant<T>,
    digest: crate::sha256_digest::Sha256Digest,
    identity: crate::paths::identity::DurableFileVersion,
    commit_fence: &mut F,
) -> Result<()>
where
    T: DeserializeOwned + Eq,
    F: FnMut() -> Result<()>,
{
    ensure!(
        digest == required.source_sha256 && identity == required.source_version,
        PARTICIPANT_CONFLICT
    );
    super::staging::verify(
        guard,
        path,
        intent.transaction_id,
        participant,
        &required.target_payload,
        required.target_version,
    )?;
    super::staging::install(
        guard,
        path,
        intent.transaction_id,
        participant,
        identity,
        required.target_version,
        &required.target_payload,
        commit_fence,
    )
}
