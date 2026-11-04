use crate::worker::tests::*;
pub(super) fn publish_agent_launch_fixture(
    fixture: &Fixture,
) -> (
    WorkerLaunchSpec,
    yolo_startup_checkpoint_contract::StartupCheckpointEvidence,
) {
    use yolo_startup_checkpoint_contract::*;
    let runtime = lock_recover(&fixture.state.record).clone();
    let launch = WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        controller_generation: runtime.controller_generation,
        socket_path: runtime.socket_path.clone(),
        auth_token: runtime.auth_token.clone(),
        command: runtime.command.clone(),
        args: runtime.args.clone(),
        cwd: runtime.cwd.clone(),
        environment: Default::default(),
        initial_input: None,
        cols: runtime.cols,
        rows: runtime.rows,
        context: None,
    };
    let checkpoint = StartupCheckpointEvidence {
        schema_version: constants::STARTUP_CHECKPOINT_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        controller_generation: runtime.controller_generation,
        launch_sha256: StartupLaunchFence::from_launch(&launch)
            .unwrap()
            .launch_sha256,
        boot_id: runtime.boot_id,
        worker: StartupProcessIdentity::new(runtime.worker_pid, runtime.worker_start_time),
        child: Some(StartupProcessIdentity::new(
            runtime.child_pid,
            runtime.child_start_time,
        )),
        stage: StartupStage::RuntimeActivation,
        phase: StartupPhase::Completed,
        status: StartupStatus::RuntimeActivated,
        successor: None,
        predecessor_runtime_sha256: None,
        pending_pty_custody_rollover: None,
    };
    atomic_write_json(
        &fixture.state.paths.worker_launch(runtime.terminal_id),
        &launch,
    )
    .unwrap();
    atomic_write_json(
        &fixture
            .state
            .paths
            .worker_startup_checkpoint(runtime.terminal_id),
        &checkpoint,
    )
    .unwrap();
    (launch, checkpoint)
}

pub(super) fn capture_agent_launch_fence(fixture: &Fixture) -> Result<agent_launch_fence::AgentLaunchFence> {
    agent_launch_fence::AgentLaunchFence::capture(
        fixture.state.namespace.lease(),
        &fixture.state.paths,
        &lock_recover(&fixture.state.record),
    )
}
