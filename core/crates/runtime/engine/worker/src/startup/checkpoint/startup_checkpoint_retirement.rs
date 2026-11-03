use super::constants::*;
use super::{
    StartupStatus, fence, path_io, settlement::require_checkpoint_custody_gone, validation,
};
use crate::model::{RuntimeRecord, WorkerLaunchSpec};
use crate::paths::Paths;
use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerStartupCheckpointPreparation {
    Absent,
    Prepared,
    AlreadyPrepared,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerStartupCheckpointReplayDisposition {
    Absent,
    Authorized,
    PreparationRequired,
    Conflicting,
}

pub fn prepare_worker_startup_checkpoint_for_launch(
    paths: &Paths,
    successor: &WorkerLaunchSpec,
    predecessor: Option<&RuntimeRecord>,
) -> Result<WorkerStartupCheckpointPreparation> {
    prepare_with(paths, successor, predecessor, || Ok(()))
}

pub fn worker_startup_checkpoint_replay_disposition(
    paths: &Paths,
    launch: &WorkerLaunchSpec,
) -> Result<WorkerStartupCheckpointReplayDisposition> {
    let requested = super::StartupLaunchFence::from_launch(launch)?;
    let Some((evidence, _)) = path_io::read_path(paths, launch.terminal_id)? else {
        return Ok(WorkerStartupCheckpointReplayDisposition::Absent);
    };
    validation::validate(&evidence)?;
    if evidence.terminal_id != launch.terminal_id {
        return Ok(WorkerStartupCheckpointReplayDisposition::Conflicting);
    }
    if evidence.status == StartupStatus::HostRetired {
        return Ok(if evidence.successor == Some(requested) {
            WorkerStartupCheckpointReplayDisposition::Authorized
        } else {
            WorkerStartupCheckpointReplayDisposition::Conflicting
        });
    }
    if evidence.status != StartupStatus::HostSettled
        || requested.controller_generation < evidence.controller_generation
        || require_checkpoint_custody_gone(&evidence).is_err()
    {
        return Ok(WorkerStartupCheckpointReplayDisposition::Conflicting);
    }
    Ok(WorkerStartupCheckpointReplayDisposition::PreparationRequired)
}

pub(crate) fn prepare_with<F>(
    paths: &Paths,
    successor: &WorkerLaunchSpec,
    predecessor: Option<&RuntimeRecord>,
    before_cas: F,
) -> Result<WorkerStartupCheckpointPreparation>
where
    F: FnOnce() -> Result<()>,
{
    let successor_fence = super::StartupLaunchFence::from_launch(successor)?;
    let Some((expected, identity)) = path_io::read_path(paths, successor_fence.terminal_id)? else {
        return Ok(WorkerStartupCheckpointPreparation::Absent);
    };
    validation::validate(&expected)?;
    anyhow::ensure!(
        expected.terminal_id == successor_fence.terminal_id
            && successor_fence.controller_generation >= expected.controller_generation,
        EVIDENCE_STALE_ERROR
    );
    if expected.status == StartupStatus::HostRetired {
        anyhow::ensure!(
            expected.successor == Some(successor_fence),
            EVIDENCE_STALE_ERROR
        );
        if let Some(predecessor) = predecessor {
            anyhow::ensure!(
                super::predecessor::worker_startup_checkpoint_authorizes_retained_runtime(
                    paths,
                    successor,
                    predecessor,
                )?,
                PREDECESSOR_ERROR
            );
        }
        return Ok(WorkerStartupCheckpointPreparation::AlreadyPrepared);
    }
    if let Some(predecessor) = predecessor {
        anyhow::ensure!(
            super::predecessor::matches_runtime(&expected, successor_fence, predecessor)?,
            PREDECESSOR_ERROR
        );
    } else {
        anyhow::ensure!(
            fence::from_evidence(&expected) == successor_fence
                || expected.status == StartupStatus::HostSettled,
            PREDECESSOR_ERROR
        );
    }
    require_checkpoint_custody_gone(&expected)?;
    let (confirmed, confirmed_identity) = path_io::read_path(paths, expected.terminal_id)?
        .ok_or_else(|| anyhow::anyhow!(EVIDENCE_MISSING_ERROR))?;
    validation::validate(&confirmed)?;
    anyhow::ensure!(
        confirmed == expected && confirmed_identity == identity,
        EVIDENCE_STALE_ERROR
    );
    let mut retired = confirmed;
    retired.status = StartupStatus::HostRetired;
    retired.successor = Some(successor_fence);
    retired.predecessor_runtime_sha256 = predecessor
        .map(super::predecessor::runtime_digest)
        .transpose()?;
    path_io::replace_path_with(paths, confirmed_identity, retired, |mutation, retired| {
        retired.pending_pty_custody_rollover = predecessor
            .map(|runtime| {
                super::predecessor::custody_rollover_for_runtime(mutation, paths, runtime, retired)
            })
            .transpose()?
            .flatten();
        before_cas()
    })?;
    Ok(WorkerStartupCheckpointPreparation::Prepared)
}

#[cfg(test)]
pub(crate) fn prepare_with_test_hook<F>(
    paths: &Paths,
    successor: &WorkerLaunchSpec,
    predecessor: Option<&RuntimeRecord>,
    before_cas: F,
) -> Result<WorkerStartupCheckpointPreparation>
where
    F: FnOnce() -> Result<()>,
{
    prepare_with(paths, successor, predecessor, before_cas)
}
