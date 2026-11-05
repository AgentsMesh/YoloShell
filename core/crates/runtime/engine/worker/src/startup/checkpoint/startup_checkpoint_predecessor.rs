use super::{
    StartupCheckpointEvidence, StartupLaunchFence, StartupStatus, constants::*, fence, path_io,
    settlement::require_checkpoint_custody_gone, validation,
};
use crate::model::{RuntimeRecord, WorkerLaunchSpec};
use crate::paths::Paths;
use crate::sha256_digest::Sha256Digest;
use anyhow::Result;

pub(crate) fn runtime_digest(runtime: &RuntimeRecord) -> Result<Sha256Digest> {
    Sha256Digest::capture(&(PREDECESSOR_RUNTIME_DIGEST_DOMAIN, runtime))
}

pub(crate) fn custody_rollover_for_runtime(
    mutation: &super::mutation::CheckpointMutation<'_>,
    paths: &Paths,
    runtime: &RuntimeRecord,
    checkpoint: &StartupCheckpointEvidence,
) -> Result<Option<super::StartupPtyCustodyRollover>> {
    let path = paths.pty_custody(runtime.terminal_id);
    let Some((custody, _, identity)) = mutation
        .read_optional_managed_with_versions::<crate::pty_custody::CustodyDurableRecord>(&path)?
    else {
        return Ok(None);
    };
    anyhow::ensure!(custody.is_valid(), PREDECESSOR_CUSTODY_ERROR);
    let custody_sha256 = crate::pty_custody::durable_record_digest(&custody)?;
    let inherited = fence::matches_runtime(checkpoint, runtime)?
        && checkpoint
            .pending_pty_custody_rollover
            .as_ref()
            .is_some_and(|rollover| {
                rollover.source_version == identity && rollover.source_sha256 == custody_sha256
            });
    anyhow::ensure!(
        custody.runtime.matches_record(runtime) || inherited,
        PREDECESSOR_CUSTODY_ERROR
    );
    Ok(Some(super::StartupPtyCustodyRollover {
        source_version: identity,
        source_sha256: custody_sha256,
    }))
}

pub(crate) fn matches_runtime(
    evidence: &StartupCheckpointEvidence,
    successor: StartupLaunchFence,
    runtime: &RuntimeRecord,
) -> Result<bool> {
    Ok(fence::matches_runtime(evidence, runtime)?
        && successor.controller_generation >= runtime.controller_generation)
}

pub fn worker_startup_checkpoint_authorizes_retained_runtime(
    paths: &Paths,
    successor: &WorkerLaunchSpec,
    retained: &RuntimeRecord,
) -> Result<bool> {
    let successor = StartupLaunchFence::from_launch(successor)?;
    let Some((evidence, _)) = path_io::read_path(paths, successor.terminal_id)? else {
        return Ok(false);
    };
    validation::validate(&evidence)?;
    let runtime_matches = matches_runtime(&evidence, successor, retained)?;
    let runtime_digest = runtime_digest(retained)?;
    if evidence.status != StartupStatus::HostRetired
        || evidence.successor != Some(successor)
        || evidence.predecessor_runtime_sha256 != Some(runtime_digest)
        || !runtime_matches
    {
        return Ok(false);
    }
    Ok(require_checkpoint_custody_gone(&evidence).is_ok())
}
