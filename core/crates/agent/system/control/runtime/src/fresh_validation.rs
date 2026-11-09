use crate::constants::INVALID_INITIALIZATION;
use anyhow::{Result, ensure};
use yolo_application_support::constants::{
    RUNTIME_RECORD_SCHEMA_VERSION, SESSION_MANIFEST_SCHEMA_VERSION,
};
use yolo_terminal_model::{
    BootstrapDelivery, RuntimeRecord, TerminalManifest, WorkerLaunchSpec,
    normalize_terminal_columns, normalize_terminal_rows,
};
use yolo_startup_checkpoint_contract::{
    self as checkpoint, StartupCheckpointEvidence, StartupLaunchFence, StartupPhase, StartupStage,
    StartupStatus,
};

pub(crate) fn validate(
    current: &RuntimeRecord,
    durable: &RuntimeRecord,
    launch: &WorkerLaunchSpec,
    evidence: &StartupCheckpointEvidence,
    manifest: &TerminalManifest,
) -> Result<()> {
    ensure!(
        current.schema_version == RUNTIME_RECORD_SCHEMA_VERSION
            && manifest.schema_version == SESSION_MANIFEST_SCHEMA_VERSION
            && manifest.terminal_id == current.terminal_id
            && !manifest.archived,
        INVALID_INITIALIZATION
    );
    current.validate_update_from(durable)?;
    ensure!(
        durable.bootstrap_delivery == BootstrapDelivery::Pending
            && current.bootstrap_delivery.is_serviceable(),
        INVALID_INITIALIZATION
    );
    checkpoint::validation::validate(evidence)?;
    ensure!(
        evidence.schema_version == checkpoint::constants::STARTUP_CHECKPOINT_SCHEMA_VERSION
            && evidence.status == StartupStatus::Active
            && evidence.stage == StartupStage::RuntimeActivation
            && evidence.phase == StartupPhase::Entered
            && evidence.controller_generation == current.controller_generation
            && checkpoint::matches_runtime(evidence, current)?,
        INVALID_INITIALIZATION
    );
    ensure!(
        checkpoint::from_evidence(evidence) == StartupLaunchFence::from_launch(launch)?,
        INVALID_INITIALIZATION
    );
    ensure!(
        launch.terminal_id == current.terminal_id
            && launch.runtime_epoch == current.runtime_epoch
            && launch.worker_id == current.worker_id
            && launch.controller_generation == current.controller_generation
            && launch.socket_path == current.socket_path
            && launch.auth_token == current.auth_token
            && launch.command == current.command
            && launch.args == current.args
            && launch.cwd == current.cwd
            && normalize_terminal_columns(launch.cols) == current.cols
            && normalize_terminal_rows(launch.rows) == current.rows,
        INVALID_INITIALIZATION
    );
    Ok(())
}
