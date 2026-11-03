use super::constants::*;
use super::durable_io::DurableRead;
use super::guard::SessionDurableMutationGuard;
use super::preparation_authority::LoadedPreparationAuthority;
use super::preparation_schema::ParticipantPreparation;
use crate::paths::Paths;
use crate::paths::identity::DurableFileVersion;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;
use uuid::Uuid;

pub(super) fn recover<T>(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    authority: &LoadedPreparationAuthority,
    source_path: &Path,
    transaction_id: Uuid,
    participant: &str,
    plan: &ParticipantPreparation,
) -> Result<()>
where
    T: DeserializeOwned + Serialize,
{
    let stage = super::staging::entry_for_cleanup(guard, source_path, transaction_id, participant)?;
    let prepared = super::receipt::prepared_path(guard, source_path, transaction_id, participant)?;
    let installed = super::receipt::path(guard, source_path, transaction_id, participant)?;
    match plan {
        ParticipantPreparation::Absent => {
            super::preparation_recovery_io::require_all_missing(&[stage, prepared, installed])
        }
        ParticipantPreparation::Replace {
            source_version,
            source_sha256,
            target_sha256,
        } => recover_replace::<T>(
            paths,
            guard,
            authority,
            source_path,
            *source_version,
            *source_sha256,
            *target_sha256,
            stage,
            prepared,
            installed,
            transaction_id,
            participant,
        ),
    }
}

pub(super) fn verify_quiescent<T>(
    guard: &SessionDurableMutationGuard<'_>,
    source_path: &Path,
    transaction_id: Uuid,
    participant: &str,
    plan: &ParticipantPreparation,
) -> Result<()>
where
    T: DeserializeOwned + Serialize,
{
    let helpers = [
        super::staging::entry_for_cleanup(guard, source_path, transaction_id, participant)?,
        super::receipt::prepared_path(guard, source_path, transaction_id, participant)?,
        super::receipt::path(guard, source_path, transaction_id, participant)?,
    ];
    super::preparation_recovery_io::require_all_missing(&helpers)?;
    match plan {
        ParticipantPreparation::Absent => ensure!(
            guard.namespace().file_is_missing(source_path)?,
            PREPARATION_SOURCE_CHANGED
        ),
        ParticipantPreparation::Replace {
            source_version,
            source_sha256,
            ..
        } => {
            let (source, identity): (T, _) = guard
                .namespace()
                .read_json_with_durable_version(source_path)?;
            ensure!(
                identity == *source_version
                    && crate::sha256_digest::Sha256Digest::capture(&source)? == *source_sha256,
                PREPARATION_SOURCE_CHANGED
            );
        }
    }
    guard.verify_session_directory()
}

#[allow(clippy::too_many_arguments)]
fn recover_replace<T>(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    authority: &LoadedPreparationAuthority,
    source_path: &Path,
    source_version: DurableFileVersion,
    source_sha256: crate::sha256_digest::Sha256Digest,
    target_sha256: crate::sha256_digest::Sha256Digest,
    stage: PrivateFileEntry,
    prepared: PrivateFileEntry,
    installed: PrivateFileEntry,
    transaction_id: Uuid,
    participant: &str,
) -> Result<()>
where
    T: DeserializeOwned + Serialize,
{
    let installed_missing = installed.is_missing()?;
    let prepared_missing = prepared.is_missing()?;
    let stage_missing = stage.is_missing()?;
    ensure!(installed_missing, PREPARATION_ARTIFACT_CHANGED);
    if prepared_missing && stage_missing {
        return super::preparation_recovery_io::verify_quiescent::<T>(
            paths,
            guard,
            authority,
            source_path,
            source_version,
            source_sha256,
            target_sha256,
        );
    }
    super::preparation_recovery_io::fence_source::<T>(
        paths,
        guard,
        authority,
        source_path,
        source_version,
        source_sha256,
    )?;
    let stage_loaded = super::preparation_recovery_io::load_stage::<T>(
        guard,
        &stage,
        transaction_id,
        target_sha256,
    )?;
    let receipt = super::receipt::read_prepared(guard, source_path, transaction_id, participant)?;
    super::preparation_recovery_io::validate_pair(
        stage_loaded.as_ref(),
        receipt.as_ref(),
        source_version,
    )?;
    if let Some((_, identity)) = receipt {
        super::preparation_recovery_io::remove_exact::<T>(
            paths,
            guard,
            authority,
            source_path,
            source_version,
            source_sha256,
            &prepared,
            identity,
        )?;
    }
    if let Some((_, identity)) = stage_loaded {
        super::preparation_recovery_io::remove_exact::<T>(
            paths,
            guard,
            authority,
            source_path,
            source_version,
            source_sha256,
            &stage,
            identity,
        )?;
    }
    Ok(())
}
