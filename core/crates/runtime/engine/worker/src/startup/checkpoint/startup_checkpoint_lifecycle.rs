use super::{StartupCheckpointEvidence, StartupStatus, constants, fence, path_io, validation};
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use anyhow::Result;
use uuid::Uuid;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

pub fn worker_startup_checkpoint_allows_lifecycle_mutation(
    paths: &Paths,
    terminal_id: Uuid,
    runtime: Option<&RuntimeRecord>,
) -> Result<bool> {
    let Some((evidence, _)) = path_io::read_path(paths, terminal_id)? else {
        return Ok(true);
    };
    allows(&evidence, terminal_id, runtime)
}

pub fn worker_startup_checkpoint_allows_guardian_retirement(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    runtime: &RuntimeRecord,
) -> Result<bool> {
    guard.require_no_pending_intent()?;
    let path = paths.worker_startup_checkpoint(runtime.terminal_id);
    let Some((evidence, _)) = guard.read_optional_with_identity(&path)? else {
        return Ok(true);
    };
    let result = allows(&evidence, runtime.terminal_id, Some(runtime));
    guard.require_no_pending_intent()?;
    result
}

pub(crate) fn allows(
    evidence: &StartupCheckpointEvidence,
    terminal_id: Uuid,
    runtime: Option<&RuntimeRecord>,
) -> Result<bool> {
    validation::validate(evidence)?;
    anyhow::ensure!(evidence.terminal_id == terminal_id, constants::STALE_ERROR);
    match evidence.status {
        StartupStatus::Active | StartupStatus::HostRetired => Ok(false),
        StartupStatus::RuntimeActivated => runtime
            .map(|runtime| fence::matches_runtime(evidence, runtime))
            .transpose()
            .map(|matched| matched == Some(true)),
        StartupStatus::HostSettled => Ok(true),
    }
}
