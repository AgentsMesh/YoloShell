use super::constants::*;
use super::{
    StartupCheckpointEvidence, StartupPhase, StartupStage, StartupStatus, path_io, validation,
};
use crate::paths::Paths;
use crate::process::ProcessIdentityRelation;
use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerStartupCheckpointSettlement {
    Absent,
    AlreadySettled,
    Settled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerStartupCheckpointPolicy {
    RequireAbsent,
    SettlePresent,
}

pub fn settle_worker_startup_checkpoint_for_launch(
    paths: &Paths,
    launch: &crate::model::WorkerLaunchSpec,
    policy: WorkerStartupCheckpointPolicy,
) -> Result<WorkerStartupCheckpointSettlement> {
    let requested = super::StartupLaunchFence::from_launch(launch)?;
    let Some(expected) = read_worker_startup_checkpoint(paths, launch.terminal_id)? else {
        return Ok(WorkerStartupCheckpointSettlement::Absent);
    };
    anyhow::ensure!(
        policy == WorkerStartupCheckpointPolicy::SettlePresent,
        EVIDENCE_PRESENT_ERROR
    );
    if expected.status == StartupStatus::HostRetired {
        anyhow::ensure!(expected.successor == Some(requested), EVIDENCE_STALE_ERROR);
        return Ok(WorkerStartupCheckpointSettlement::AlreadySettled);
    }
    anyhow::ensure!(
        super::fence::from_evidence(&expected) == requested,
        EVIDENCE_STALE_ERROR
    );
    if expected.status == StartupStatus::HostSettled {
        return Ok(WorkerStartupCheckpointSettlement::AlreadySettled);
    }
    settle_worker_startup_checkpoint_if_gone(paths, &expected)?;
    Ok(WorkerStartupCheckpointSettlement::Settled)
}

pub(crate) fn read_worker_startup_checkpoint(
    paths: &Paths,
    terminal_id: uuid::Uuid,
) -> Result<Option<StartupCheckpointEvidence>> {
    let Some((evidence, _)) = path_io::read_path(paths, terminal_id)? else {
        return Ok(None);
    };
    validation::validate(&evidence)?;
    anyhow::ensure!(evidence.terminal_id == terminal_id, EVIDENCE_STALE_ERROR);
    Ok(Some(evidence))
}

pub(crate) fn settle_worker_startup_checkpoint_if_gone(
    paths: &Paths,
    expected: &StartupCheckpointEvidence,
) -> Result<()> {
    validation::validate(expected)?;
    anyhow::ensure!(
        matches!(
            expected.status,
            StartupStatus::Active | StartupStatus::RuntimeActivated
        ),
        EVIDENCE_STALE_ERROR
    );
    require_checkpoint_custody_gone(expected)?;
    let (stored, identity) = path_io::read_path(paths, expected.terminal_id)?
        .ok_or_else(|| anyhow::anyhow!(EVIDENCE_MISSING_ERROR))?;
    validation::validate(&stored)?;
    anyhow::ensure!(stored == *expected, EVIDENCE_STALE_ERROR);
    let mut settled = stored;
    settled.status = StartupStatus::HostSettled;
    validation::validate(&settled)?;
    path_io::replace_path(paths, identity, &settled)
}

pub(crate) fn require_checkpoint_custody_gone(expected: &StartupCheckpointEvidence) -> Result<()> {
    require_checkpoint_custody_gone_on_boot(expected, &crate::process::boot_id())
}

pub(crate) fn require_checkpoint_custody_gone_on_boot(
    expected: &StartupCheckpointEvidence,
    current_boot_id: &str,
) -> Result<()> {
    validation::validate(expected)?;
    match crate::process::boot_identity_relation(&expected.boot_id, current_boot_id) {
        crate::process::BootIdentityRelation::Different => return Ok(()),
        crate::process::BootIdentityRelation::Unknown => {
            anyhow::bail!(PROCESS_LIVE_ERROR)
        }
        crate::process::BootIdentityRelation::Same => {}
    }
    anyhow::ensure!(
        expected.stage != StartupStage::ChildProcess || expected.phase != StartupPhase::Entered,
        CHILD_CUSTODY_AMBIGUOUS_ERROR
    );
    require_gone(expected.worker)?;
    if let Some(child) = expected.child {
        require_gone(child)?;
    }
    Ok(())
}

pub(crate) fn require_gone(identity: super::StartupProcessIdentity) -> Result<()> {
    anyhow::ensure!(
        crate::process::process_identity_relation(identity.pid, identity.start_time)
            == ProcessIdentityRelation::Gone,
        PROCESS_LIVE_ERROR
    );
    Ok(())
}
