use super::{StartupCheckpointEvidence, StartupLaunchFence, StartupProcessIdentity};
use anyhow::Result;
use yolo_application_support::constants::{
    RUNTIME_RECORD_SCHEMA_VERSION, WORKER_LAUNCH_SCHEMA_VERSION,
};
use yolo_terminal_model::{RuntimeRecord, WorkerLaunchSpec};

impl StartupLaunchFence {
    pub fn from_launch(launch: &WorkerLaunchSpec) -> Result<Self> {
        anyhow::ensure!(
            launch.schema_version == WORKER_LAUNCH_SCHEMA_VERSION,
            super::constants::SUCCESSOR_ERROR
        );
        let fence = Self {
            terminal_id: launch.terminal_id,
            runtime_epoch: launch.runtime_epoch,
            worker_id: launch.worker_id,
            controller_generation: launch.controller_generation,
            launch_sha256: yolo_crypto::Sha256Digest::capture(&(
                super::constants::LAUNCH_DIGEST_DOMAIN,
                launch,
            ))?,
        };
        validate(fence)?;
        Ok(fence)
    }
}

pub fn from_evidence(evidence: &StartupCheckpointEvidence) -> StartupLaunchFence {
    StartupLaunchFence {
        terminal_id: evidence.terminal_id,
        runtime_epoch: evidence.runtime_epoch,
        worker_id: evidence.worker_id,
        controller_generation: evidence.controller_generation,
        launch_sha256: evidence.launch_sha256,
    }
}

pub fn validate(fence: StartupLaunchFence) -> Result<()> {
    super::validation::validate_launch_fence(
        fence.terminal_id,
        fence.runtime_epoch,
        fence.worker_id,
        fence.controller_generation,
    )
}

pub fn matches_runtime(
    evidence: &StartupCheckpointEvidence,
    runtime: &RuntimeRecord,
) -> Result<bool> {
    anyhow::ensure!(
        runtime.schema_version == RUNTIME_RECORD_SCHEMA_VERSION,
        super::constants::PREDECESSOR_ERROR
    );
    let worker = process(runtime.worker_pid, runtime.worker_start_time)?;
    let child = process(runtime.child_pid, runtime.child_start_time)?;
    Ok(evidence.terminal_id == runtime.terminal_id
        && evidence.runtime_epoch == runtime.runtime_epoch
        && evidence.worker_id == runtime.worker_id
        && evidence.controller_generation <= runtime.controller_generation
        && evidence.boot_id == runtime.boot_id
        && evidence.worker == worker
        && evidence.child == Some(child))
}

fn process(pid: u32, start_time: u64) -> Result<StartupProcessIdentity> {
    let identity = StartupProcessIdentity::new(pid, start_time);
    super::validation::validate_process(identity)?;
    Ok(identity)
}
