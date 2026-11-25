const MISSING_CREATION_WORKER_ID: u32 = i32::MAX as u32;
const MISSING_CREATION_CHILD_ID: u32 = MISSING_CREATION_WORKER_ID - 1;
const MISSING_CREATION_RUNTIME: &str = "must-not-spawn";
const PENDING_CREATION_NAME: &str = "pending-creation";
const CREATION_RUNTIME_TEST_SETTLEMENT_TIMEOUT: Duration = Duration::from_millis(250);

const fn creation_launch_test_timeout(readiness: Duration) -> Duration {
    readiness.saturating_add(crate::session_timing::SESSION_DURABLE_STARTUP_SETTLEMENT_MARGIN)
}

fn grant_terminal_creation_settlement_budget(host: &mut Host) {
    host.worker_ready_timeout = HOST_BLOCKING_TEST_TIMEOUT;
}

fn runtime_for_creation(
    host: &Host,
    launch: &WorkerLaunchSpec,
    delivery: BootstrapDelivery,
    live: bool,
) -> RuntimeRecord {
    let (worker_pid, worker_start_time, child_pid, child_start_time) = if live {
        let worker_pid = std::process::id();
        let child_pid = unsafe { libc::getppid() as u32 };
        (
            worker_pid,
            process::process_start_time(worker_pid).unwrap(),
            child_pid,
            process::process_start_time(child_pid).unwrap(),
        )
    } else {
        (
            MISSING_CREATION_WORKER_ID,
            MISSING_PROCESS_START_TIME,
            MISSING_CREATION_CHILD_ID,
            MISSING_PROCESS_START_TIME,
        )
    };
    RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: launch.terminal_id,
        runtime_epoch: launch.runtime_epoch,
        worker_id: launch.worker_id,
        worker_pid,
        worker_start_time,
        child_pid,
        child_start_time,
        boot_id: host.boot_id.clone(),
        socket_path: launch.socket_path.clone(),
        auth_token: launch.auth_token.clone(),
        controller_generation: launch.controller_generation,
        started_at_ms: now_ms(),
        command: launch.command.clone(),
        args: launch.args.clone(),
        cwd: launch.cwd.clone(),
        cols: launch.cols,
        rows: launch.rows,
        bootstrap_delivery: delivery,
    }
}

async fn pending_creation_fixture(
    live: bool,
) -> (
    tempfile::TempDir,
    Host,
    CreateTerminal,
    CreationBinding,
    CreationLaunchIdentity,
    RuntimeRecord,
) {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join(MISSING_CREATION_RUNTIME));
    let mut host = Host::open(config).unwrap();
    host.worker_ready_timeout = HOST_BLOCKING_TEST_TIMEOUT;
    let (request, reserved, ledger_path) =
        reserve_test_creation(&host, directory.path(), PENDING_CREATION_NAME);
    host.prepare_bound_session(&reserved).await.unwrap();
    let planned = lock_recover(&host.creation_capabilities)
        .prepare_launch(&ledger_path, reserved.result)
        .unwrap();
    let binding = lock_recover(&host.creation_capabilities)
        .bindings
        .iter()
        .find(|binding| binding.result == reserved.result)
        .unwrap()
        .clone();
    let launch = planned_worker_launch(&host, &binding, &planned);
    let runtime = runtime_for_creation(&host, &launch, BootstrapDelivery::Pending, live);
    atomic_write_json(
        &host.paths.worker_launch(binding.result.terminal_id),
        &launch,
    )
    .unwrap();
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    host.worker_ready_timeout = CREATION_RUNTIME_TEST_SETTLEMENT_TIMEOUT;
    (directory, host, request, binding, planned, runtime)
}

fn spawn_creation_authentication_server(record: &RuntimeRecord) -> tokio::task::JoinHandle<()> {
    let listener = UnixListener::bind(&record.socket_path).unwrap();
    let socket_path = record.socket_path.clone();
    let record = record.clone();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &record).await;
        drop(listener);
        std::fs::remove_file(socket_path).unwrap();
    })
}

fn assert_creation_plan_phase(
    host: &Host,
    binding: &CreationBinding,
    planned: &CreationLaunchIdentity,
    phase: CreationPhase,
) {
    let ledger = lock_recover(&host.creation_capabilities);
    let stored = ledger
        .bindings
        .iter()
        .find(|entry| entry.result == binding.result)
        .unwrap();
    assert_eq!(stored.phase, phase);
    assert_eq!(stored.launch.as_ref(), Some(planned));
}

fn assert_launching_plan(host: &Host, binding: &CreationBinding, planned: &CreationLaunchIdentity) {
    assert_creation_plan_phase(host, binding, planned, CreationPhase::Launching);
}

fn assert_runtime_mismatch<F>(
    host: &Host,
    binding: &CreationBinding,
    planned: &CreationLaunchIdentity,
    runtime: &RuntimeRecord,
    mutate: F,
) where
    F: FnOnce(&mut RuntimeRecord),
{
    let mut candidate = runtime.clone();
    mutate(&mut candidate);
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &candidate,
    )
    .unwrap();
    assert_eq!(
        host.creation_runtime_evidence(binding, planned),
        CreationRuntimeEvidence::Mismatch
    );
}
