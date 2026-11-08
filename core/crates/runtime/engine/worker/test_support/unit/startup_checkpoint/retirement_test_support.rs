use super::*;
use crate::constants::RUNTIME_RECORD_SCHEMA_VERSION;
use crate::model::{BootstrapDelivery, RuntimeRecord, WorkerLaunchSpec};

pub(super) fn successor_for(fixture: &CheckpointFixture) -> WorkerLaunchSpec {
    let mut successor = fixture.launch.clone();
    successor.runtime_epoch = Uuid::new_v4();
    successor.worker_id = Uuid::new_v4();
    successor
}

pub(super) fn activated_runtime(fixture: &CheckpointFixture) -> RuntimeRecord {
    let worker = gone_process();
    let child = gone_process();
    let mut checkpoint =
        ActiveStartupCheckpoint::begin(&fixture.namespace, &fixture.paths, &fixture.launch, worker)
            .unwrap();
    for stage in STARTUP_STAGES {
        checkpoint
            .enter(&fixture.namespace, &fixture.paths, stage)
            .unwrap();
        if stage == StartupStage::RuntimeActivation {
            checkpoint
                .retire_runtime_activated(&fixture.namespace, &fixture.paths)
                .unwrap();
        } else if stage == StartupStage::ChildProcess {
            checkpoint
                .complete_child(&fixture.namespace, &fixture.paths, child)
                .unwrap();
        } else {
            checkpoint
                .complete(&fixture.namespace, &fixture.paths, stage)
                .unwrap();
        }
    }
    let runtime = runtime_for(fixture, worker, child);
    crate::paths::atomic_write_json(&fixture.paths.runtime_record(runtime.terminal_id), &runtime)
        .unwrap();
    runtime
}

pub(super) fn runtime_for(
    fixture: &CheckpointFixture,
    worker: ProcessIdentity,
    child: ProcessIdentity,
) -> RuntimeRecord {
    RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: fixture.launch.terminal_id,
        runtime_epoch: fixture.launch.runtime_epoch,
        worker_id: fixture.launch.worker_id,
        worker_pid: worker.pid,
        worker_start_time: worker.start_time,
        child_pid: child.pid,
        child_start_time: child.start_time,
        boot_id: crate::process::boot_id(),
        socket_path: fixture.launch.socket_path.clone(),
        auth_token: fixture.launch.auth_token.clone(),
        controller_generation: fixture.launch.controller_generation,
        started_at_ms: TEST_STARTED_AT_MS,
        command: fixture.launch.command.clone(),
        args: fixture.launch.args.clone(),
        cwd: fixture.launch.cwd.clone(),
        cols: fixture.launch.cols,
        rows: fixture.launch.rows,
        bootstrap_delivery: BootstrapDelivery::Delivered,
    }
}

pub(super) fn prepare(
    fixture: &CheckpointFixture,
    successor: &WorkerLaunchSpec,
    predecessor: Option<&RuntimeRecord>,
) -> anyhow::Result<WorkerStartupCheckpointPreparation> {
    prepare_worker_startup_checkpoint_for_launch(&fixture.paths, successor, predecessor)
}
