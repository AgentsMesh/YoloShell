use super::constants::PARTICIPANT_CONFLICT;
use super::guard::SessionDurableMutationGuard;
use super::preparation_authority::LoadedPreparationAuthority;
use super::preparation_capture::CapturedParticipant;
use super::schema::{OptionalParticipant, RequiredParticipant};
use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;
use uuid::Uuid;

pub(super) fn required<T>(
    paths: &crate::paths::Paths,
    guard: &SessionDurableMutationGuard<'_>,
    authority: &LoadedPreparationAuthority,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
    captured: CapturedParticipant<T>,
    cleanup: &mut super::prepare_cleanup::PrepareCleanup<'_>,
) -> Result<OptionalParticipant<T>>
where
    T: Clone + DeserializeOwned + Eq + Serialize,
{
    let CapturedParticipant::Replace {
        source,
        source_version,
        target,
    } = captured
    else {
        anyhow::bail!(PARTICIPANT_CONFLICT);
    };
    required_from_source(
        paths,
        guard,
        authority,
        path,
        transaction_id,
        participant,
        source_version,
        &source,
        &target,
        cleanup,
    )
    .map(OptionalParticipant::Replace)
}

pub(super) fn optional<T>(
    paths: &crate::paths::Paths,
    guard: &SessionDurableMutationGuard<'_>,
    authority: &LoadedPreparationAuthority,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
    captured: CapturedParticipant<T>,
    cleanup: &mut super::prepare_cleanup::PrepareCleanup<'_>,
) -> Result<OptionalParticipant<T>>
where
    T: Clone + DeserializeOwned + Eq + Serialize,
{
    match captured {
        CapturedParticipant::Absent => Ok(OptionalParticipant::Absent),
        captured => required(
            paths,
            guard,
            authority,
            path,
            transaction_id,
            participant,
            captured,
            cleanup,
        ),
    }
}

pub(super) fn required_from_source<T>(
    paths: &crate::paths::Paths,
    guard: &SessionDurableMutationGuard<'_>,
    authority: &LoadedPreparationAuthority,
    path: &Path,
    transaction_id: Uuid,
    participant: &str,
    source_version: crate::paths::identity::DurableFileVersion,
    source: &T,
    target: &T,
    cleanup: &mut super::prepare_cleanup::PrepareCleanup<'_>,
) -> Result<RequiredParticipant<T>>
where
    T: Clone + DeserializeOwned + Eq + Serialize,
{
    super::preparation_authority::fence_no_intent(paths, guard, authority)?;
    let target_version =
        super::staging_prepare::prepare(guard, path, transaction_id, participant, target, cleanup)?;
    super::preparation_authority::fence_no_intent(paths, guard, authority)?;
    #[cfg(test)]
    super::prepare_test_hook::run_after_stage()?;
    super::preparation_authority::fence_no_intent(paths, guard, authority)?;
    let prepared_receipt_version = super::receipt_prepare::prepare(
        guard,
        path,
        transaction_id,
        participant,
        target_version,
        source_version,
        cleanup,
    )?;
    super::preparation_authority::fence_no_intent(paths, guard, authority)?;
    RequiredParticipant::new(
        source_version,
        target_version,
        prepared_receipt_version,
        source,
        target,
    )
}
