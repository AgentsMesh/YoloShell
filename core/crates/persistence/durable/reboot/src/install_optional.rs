use super::constants::*;
use super::guard::SessionDurableMutationGuard;
use super::intent_io::LoadedIntent;
use super::schema::{OptionalParticipant, RebootMigrationIntent};
use crate::paths::Paths;
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

pub(super) fn install<T>(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedIntent,
    path: &Path,
    participant: &OptionalParticipant<T>,
) -> Result<()>
where
    T: Clone + DeserializeOwned + Eq + Serialize,
{
    let intent = &loaded.value;
    match participant {
        OptionalParticipant::Absent => ensure!(
            guard.namespace().file_is_missing(path)?,
            PARTICIPANT_CONFLICT
        ),
        OptionalParticipant::Replace(required) => super::install_participant::install(
            guard,
            path,
            intent,
            name(path, paths, intent)?,
            required,
            || {
                super::intent_io::verify_exact(paths, guard, loaded)?;
                guard.verify_session_directory()
            },
        )?,
    }
    Ok(())
}

pub(super) fn verify<T>(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    loaded: &LoadedIntent,
    path: &Path,
    participant: &OptionalParticipant<T>,
) -> Result<()>
where
    T: DeserializeOwned + Eq + Serialize,
{
    let intent = &loaded.value;
    super::intent_io::verify_exact(paths, guard, loaded)?;
    match participant {
        OptionalParticipant::Absent => ensure!(
            guard.namespace().file_is_missing(path)?,
            PARTICIPANT_CONFLICT
        ),
        OptionalParticipant::Replace(required) => super::install_participant::verify(
            guard,
            path,
            intent,
            name(path, paths, intent)?,
            required,
        )?,
    }
    Ok(())
}

pub(super) fn name(
    path: &Path,
    paths: &Paths,
    intent: &RebootMigrationIntent,
) -> Result<&'static str> {
    let participants = super::participant_paths::ParticipantPaths::new(paths, intent.terminal_id)?;
    if path == participants.startup_checkpoint {
        Ok(STARTUP_CHECKPOINT_PARTICIPANT)
    } else if path == participants.claude_custody {
        Ok(CLAUDE_CUSTODY_PARTICIPANT)
    } else if path == participants.claude_generation {
        Ok(CLAUDE_GENERATION_PARTICIPANT)
    } else if path == participants.pty_custody {
        Ok(PTY_CUSTODY_PARTICIPANT)
    } else {
        anyhow::bail!(PARTICIPANT_PATH_INVALID)
    }
}
