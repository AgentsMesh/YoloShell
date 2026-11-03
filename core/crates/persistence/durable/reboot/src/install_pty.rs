use super::constants::{PARTICIPANT_CONFLICT, PTY_CUSTODY_PARTICIPANT};
use super::durable_io::DurableRead;
use super::guard::SessionDurableMutationGuard;
use super::intent_io::LoadedIntent;
use super::schema::{OptionalParticipant, RebootMigrationIntent, RequiredParticipant};
use crate::paths::Paths;
use anyhow::{Result, ensure};
use std::path::Path;

type Custody = crate::pty_custody::CustodyDurableRecord;

pub(super) fn install(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedIntent,
    path: &Path,
    participant: &OptionalParticipant<Custody>,
) -> Result<()> {
    let intent = &loaded.value;
    super::intent_io::verify_exact(paths, guard, loaded)?;
    let OptionalParticipant::Replace(required) = participant else {
        ensure!(
            guard.namespace().file_is_missing(path)?,
            PARTICIPANT_CONFLICT
        );
        return Ok(());
    };
    super::install_participant::install(
        guard,
        path,
        intent,
        PTY_CUSTODY_PARTICIPANT,
        required,
        || {
            super::intent_io::verify_exact(paths, guard, loaded)?;
            verify_source_object(guard, path, intent, required)?;
            guard.verify_session_directory()
        },
    )
}

pub(super) fn verify_source_object(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    intent: &RebootMigrationIntent,
    required: &RequiredParticipant<Custody>,
) -> Result<()> {
    let Some(expected) = required.source_object_sha256 else {
        return Ok(());
    };
    let (current, identity): (Custody, _) =
        guard.namespace().read_json_with_durable_version(path)?;
    let source = if identity.same_content_version(required.source_version) {
        current
    } else {
        ensure!(
            identity.same_content_version(required.target_version),
            PARTICIPANT_CONFLICT
        );
        let (source, displaced) = super::staging_displaced::read(
            guard,
            path,
            intent.transaction_id,
            PTY_CUSTODY_PARTICIPANT,
        )?;
        ensure!(
            displaced.same_content_version(required.source_version),
            PARTICIPANT_CONFLICT
        );
        source
    };
    ensure!(
        crate::pty_custody::durable_record_digest(&source)? == expected,
        PARTICIPANT_CONFLICT
    );
    Ok(())
}
