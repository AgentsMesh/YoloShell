use super::*;
use crate::worker::startup::startup_checkpoint::{ActiveStartupCheckpoint, StartupStage};

const COMPLETED_STAGES: &[StartupStage] = &[
    StartupStage::StartupContainment,
    StartupStage::WorkerSocket,
    StartupStage::AgentRuntime,
    StartupStage::TerminalJournal,
    StartupStage::Pty,
    StartupStage::ChildProcess,
    StartupStage::BootstrapGate,
    StartupStage::RuntimeRecord,
    StartupStage::SecondaryCustody,
    StartupStage::CommandExec,
    StartupStage::BootstrapInput,
];

/// Seed real startup evidence using the production checkpoint state machine.
/// The retained Worker and PTY identities come from the existing live fixture.
pub(super) fn initialize(fixture: &Fixture) {
    let state = &fixture.state;
    state.ready.store(false, Ordering::Release);
    let record = lock_recover(&state.record).clone();
    let launch = WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: record.terminal_id,
        runtime_epoch: record.runtime_epoch,
        worker_id: record.worker_id,
        socket_path: record.socket_path.clone(),
        auth_token: record.auth_token.clone(),
        controller_generation: record.controller_generation,
        command: record.command.clone(),
        args: record.args.clone(),
        cwd: record.cwd.clone(),
        environment: Default::default(),
        initial_input: None,
        cols: record.cols,
        rows: record.rows,
        context: None,
    };
    let mut durable = record.clone();
    durable.bootstrap_delivery = BootstrapDelivery::Pending;
    let namespace = state.namespace.lease();
    namespace
        .write_json(&state.paths.runtime_record(record.terminal_id), &durable)
        .unwrap();
    namespace
        .write_json(&state.paths.worker_launch(record.terminal_id), &launch)
        .unwrap();
    let mut checkpoint = {
        let _lifecycle = state
            .paths
            .try_acquire_session_lifecycle_lock(record.terminal_id)
            .unwrap()
            .unwrap();
        ActiveStartupCheckpoint::begin(
            &state.namespace,
            &state.paths,
            &launch,
            process::ProcessIdentity {
                pid: record.worker_pid,
                start_time: record.worker_start_time,
            },
        )
        .unwrap()
    };
    for &stage in COMPLETED_STAGES {
        checkpoint
            .enter(&state.namespace, &state.paths, stage)
            .unwrap();
        if stage == StartupStage::ChildProcess {
            checkpoint
                .complete_child(
                    &state.namespace,
                    &state.paths,
                    process::ProcessIdentity {
                        pid: record.child_pid,
                        start_time: record.child_start_time,
                    },
                )
                .unwrap();
        } else {
            checkpoint
                .complete(&state.namespace, &state.paths, stage)
                .unwrap();
        }
    }
    checkpoint
        .enter(
            &state.namespace,
            &state.paths,
            StartupStage::RuntimeActivation,
        )
        .unwrap();
    state.initialize_native_control(&record);
    assert!(state.native_control_available());
    namespace
        .remove_file(&state.paths.worker_launch(record.terminal_id))
        .unwrap();
    namespace
        .write_json(&state.paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    checkpoint
        .retire_runtime_activated(&state.namespace, &state.paths)
        .unwrap();
    state.ready.store(true, Ordering::Release);
}
