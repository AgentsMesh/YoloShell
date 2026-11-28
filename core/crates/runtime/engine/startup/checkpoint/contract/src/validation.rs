use super::constants::*;
use super::{
    StartupCheckpointEvidence, StartupPhase, StartupProcessIdentity, StartupStage, StartupStatus,
};
use anyhow::Result;
use uuid::Uuid;

pub fn validate(evidence: &StartupCheckpointEvidence) -> Result<()> {
    anyhow::ensure!(
        evidence.schema_version == STARTUP_CHECKPOINT_SCHEMA_VERSION,
        SCHEMA_ERROR
    );
    anyhow::ensure!(
        evidence.pending_pty_custody_rollover.is_none()
            || evidence.predecessor_runtime_sha256.is_some(),
        SCHEMA_ERROR
    );
    validate_launch_fence(
        evidence.terminal_id,
        evidence.runtime_epoch,
        evidence.worker_id,
        evidence.controller_generation,
    )?;
    anyhow::ensure!(
        !evidence.boot_id.is_empty()
            && evidence.boot_id.len() <= yolo_process_identity::MAX_BOOT_IDENTITY_BYTES,
        BOOT_ID_ERROR
    );
    validate_process(evidence.worker)?;
    if let Some(child) = evidence.child {
        validate_process(child)?;
        anyhow::ensure!(child.pid != evidence.worker.pid, CHILD_ERROR);
    }
    validate_successor(evidence)?;
    validate_child_progress(evidence)?;
    validate_terminal_progress(evidence)
}

pub fn validate_launch_fence(
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    worker_id: Uuid,
    controller_generation: u64,
) -> Result<()> {
    anyhow::ensure!(
        !terminal_id.is_nil()
            && !runtime_epoch.is_nil()
            && !worker_id.is_nil()
            && runtime_epoch != worker_id
            && controller_generation != 0,
        IDENTITY_ERROR
    );
    Ok(())
}

pub fn validate_process(identity: StartupProcessIdentity) -> Result<()> {
    anyhow::ensure!(
        identity.pid != 0 && identity.pid <= MAX_PROCESS_ID && identity.start_time != 0,
        PROCESS_ERROR
    );
    Ok(())
}

fn validate_successor(evidence: &StartupCheckpointEvidence) -> Result<()> {
    match (evidence.status, evidence.successor) {
        (StartupStatus::HostRetired, Some(successor)) => {
            validate_launch_fence(
                successor.terminal_id,
                successor.runtime_epoch,
                successor.worker_id,
                successor.controller_generation,
            )?;
            anyhow::ensure!(successor.terminal_id == evidence.terminal_id, SUCCESSOR_ERROR);
        }
        (StartupStatus::HostRetired, None) => anyhow::bail!(SUCCESSOR_ERROR),
        (_, None) => {}
        (_, Some(_)) => anyhow::bail!(SUCCESSOR_ERROR),
    }
    Ok(())
}

fn validate_child_progress(evidence: &StartupCheckpointEvidence) -> Result<()> {
    let child_required = match evidence.stage {
        StartupStage::LaunchContract
        | StartupStage::StartupContainment
        | StartupStage::WorkerSocket
        | StartupStage::AgentRuntime
        | StartupStage::TerminalJournal
        | StartupStage::Pty => false,
        StartupStage::ChildProcess => evidence.phase == StartupPhase::Completed,
        StartupStage::BootstrapGate
        | StartupStage::RuntimeRecord
        | StartupStage::SecondaryCustody
        | StartupStage::CommandExec
        | StartupStage::BootstrapInput
        | StartupStage::RuntimeActivation => true,
    };
    anyhow::ensure!(evidence.child.is_some() == child_required, CHILD_ERROR);
    Ok(())
}

fn validate_terminal_progress(evidence: &StartupCheckpointEvidence) -> Result<()> {
    match evidence.status {
        StartupStatus::Active => anyhow::ensure!(
            evidence.stage != StartupStage::RuntimeActivation
                || evidence.phase == StartupPhase::Entered,
            PROGRESS_ERROR
        ),
        StartupStatus::RuntimeActivated => anyhow::ensure!(
            evidence.stage == StartupStage::RuntimeActivation
                && evidence.phase == StartupPhase::Completed,
            PROGRESS_ERROR
        ),
        StartupStatus::HostSettled | StartupStatus::HostRetired => {}
    }
    anyhow::ensure!(
        evidence.stage != StartupStage::LaunchContract || evidence.phase == StartupPhase::Completed,
        PROGRESS_ERROR
    );
    Ok(())
}

pub fn require_next_stage(
    evidence: &StartupCheckpointEvidence,
    requested: StartupStage,
) -> Result<()> {
    anyhow::ensure!(
        evidence.status == StartupStatus::Active
            && evidence.phase == StartupPhase::Completed
            && next_stage(evidence.stage) == Some(requested),
        TRANSITION_ERROR
    );
    Ok(())
}

pub fn require_entered_stage(
    evidence: &StartupCheckpointEvidence,
    requested: StartupStage,
    binds_child: bool,
) -> Result<()> {
    anyhow::ensure!(
        evidence.status == StartupStatus::Active
            && evidence.stage == requested
            && evidence.phase == StartupPhase::Entered
            && (requested == StartupStage::ChildProcess) == binds_child,
        TRANSITION_ERROR
    );
    Ok(())
}

fn next_stage(stage: StartupStage) -> Option<StartupStage> {
    Some(match stage {
        StartupStage::LaunchContract => StartupStage::StartupContainment,
        StartupStage::StartupContainment => StartupStage::WorkerSocket,
        StartupStage::WorkerSocket => StartupStage::AgentRuntime,
        StartupStage::AgentRuntime => StartupStage::TerminalJournal,
        StartupStage::TerminalJournal => StartupStage::Pty,
        StartupStage::Pty => StartupStage::ChildProcess,
        StartupStage::ChildProcess => StartupStage::BootstrapGate,
        StartupStage::BootstrapGate => StartupStage::RuntimeRecord,
        StartupStage::RuntimeRecord => StartupStage::SecondaryCustody,
        StartupStage::SecondaryCustody => StartupStage::CommandExec,
        StartupStage::CommandExec => StartupStage::BootstrapInput,
        StartupStage::BootstrapInput => StartupStage::RuntimeActivation,
        StartupStage::RuntimeActivation => return None,
    })
}
