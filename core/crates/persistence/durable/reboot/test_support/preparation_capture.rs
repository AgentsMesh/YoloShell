use super::constants::PARTICIPANT_CONFLICT;
use super::guard::SessionDurableMutationGuard;
use super::preparation_schema::ParticipantPreparation;
use crate::paths::identity::DurableFileVersion;
use crate::sha256_digest::Sha256Digest;
use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

pub(super) enum CapturedParticipant<T> {
    Absent,
    Replace {
        source: T,
        source_version: DurableFileVersion,
        target: T,
    },
}

impl<T> CapturedParticipant<T>
where
    T: Serialize,
{
    pub(super) fn plan(&self) -> Result<ParticipantPreparation> {
        match self {
            Self::Absent => Ok(ParticipantPreparation::Absent),
            Self::Replace {
                source,
                source_version,
                target,
            } => Ok(ParticipantPreparation::Replace {
                source_version: *source_version,
                source_sha256: Sha256Digest::capture(source)?,
                target_sha256: Sha256Digest::capture(target)?,
            }),
        }
    }
}

pub(super) fn required<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    target: T,
) -> Result<CapturedParticipant<T>>
where
    T: DeserializeOwned,
{
    let Some((source, source_version)) = super::durable_io::optional(guard, path)? else {
        anyhow::bail!(PARTICIPANT_CONFLICT);
    };
    Ok(CapturedParticipant::Replace {
        source,
        source_version,
        target,
    })
}

pub(super) fn optional<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    target: Option<T>,
) -> Result<CapturedParticipant<T>>
where
    T: DeserializeOwned,
{
    let source = super::durable_io::optional(guard, path)?;
    match (source, target) {
        (None, None) => Ok(CapturedParticipant::Absent),
        (Some((source, source_version)), Some(target)) => Ok(CapturedParticipant::Replace {
            source,
            source_version,
            target,
        }),
        _ => Err(anyhow::anyhow!(PARTICIPANT_CONFLICT)),
    }
}
