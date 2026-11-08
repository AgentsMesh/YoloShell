use super::*;

pub(super) fn fixture() -> (
    ChildGuard,
    ChildGuard,
    DetachedRuntimeGuard,
    yolo_runtime::TerminalView,
) {
    let (host, mut victims, guard, mut views) = inventory_fixture(SINGLE_RUNTIME_COUNT);
    (host, victims.pop().unwrap(), guard, views.pop().unwrap())
}

pub(super) fn inventory_fixture(
    count: usize,
) -> (
    ChildGuard,
    Vec<ChildGuard>,
    DetachedRuntimeGuard,
    Vec<yolo_runtime::TerminalView>,
) {
    let home = tempfile::tempdir().unwrap().keep();
    let paths = Paths::new(&home);
    let mut victims = Vec::with_capacity(count);
    let mut views = Vec::with_capacity(count);
    for _ in 0..count {
        let victim = ChildGuard::new(
            Command::new(TEST_COMMAND)
                .arg(TEST_ARGUMENT)
                .spawn()
                .unwrap(),
        );
        let process = ProcessIdentity::capture(victim.id()).unwrap();
        let record = record(&paths, process);
        yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
            .unwrap();
        victims.push(victim);
        views.push(view(&record));
    }
    let host = ChildGuard::new(
        Command::new(TEST_COMMAND)
            .arg(TEST_ARGUMENT)
            .spawn()
            .unwrap(),
    );
    let host_identity = ProcessIdentity::capture(host.id()).unwrap();
    let mut guard = DetachedRuntimeGuard::from_peer(&home, host_identity);
    guard
        .verify_status(&status(paths.home(), host_identity.pid))
        .unwrap();
    assert_eq!(
        guard.cleanup_authority_counts_for_test(),
        (NO_RUNTIME_AUTHORITY, count, NO_RUNTIME_AUTHORITY)
    );
    (host, victims, guard, views)
}

pub(super) fn settle(
    host: &mut ChildGuard,
    victim: &mut ChildGuard,
    guard: &mut DetachedRuntimeGuard,
) {
    guard.disarm_for_test();
    settle_all(std::slice::from_mut(victim));
    settle_child(host);
}

pub(super) fn settle_all(victims: &mut [ChildGuard]) {
    for victim in victims {
        settle_child(victim);
    }
}

pub(super) fn settle_child(child: &mut ChildGuard) {
    child.kill().unwrap();
    child.wait().unwrap();
}

fn record(paths: &Paths, process: ProcessIdentity) -> RuntimeRecord {
    let terminal_id = uuid::Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    RuntimeRecord {
        schema_version: yolo_runtime::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: terminal_id,
        runtime_epoch: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        worker_pid: process.pid,
        worker_start_time: process.start_time,
        child_pid: process.pid,
        child_start_time: process.start_time,
        boot_id: TEST_BOOT_ID.into(),
        socket_path: paths.worker_socket(terminal_id).unwrap(),
        auth_token: TEST_AUTH_TOKEN.into(),
        controller_generation: TEST_CONTROLLER_GENERATION,
        started_at_ms: i64::default(),
        command: TEST_COMMAND.into(),
        args: vec![TEST_ARGUMENT.into()],
        cwd: paths.home().to_path_buf(),
        cols: yolo_runtime::model::MIN_TERMINAL_COLUMNS,
        rows: yolo_runtime::model::MIN_TERMINAL_ROWS,
        bootstrap_delivery: BootstrapDelivery::NotRequired,
    }
}

fn view(record: &RuntimeRecord) -> yolo_runtime::TerminalView {
    yolo_runtime::TerminalView {
        manifest: yolo_runtime::TerminalManifest {
            schema_version: yolo_runtime::constants::SESSION_MANIFEST_SCHEMA_VERSION,
            terminal_id: record.terminal_id,
            name: TEST_SESSION_NAME.into(),
            command: record.command.clone(),
            args: record.args.clone(),
            cwd: record.cwd.clone(),
            cols: record.cols,
            rows: record.rows,
            created_at_ms: record.started_at_ms,
            updated_at_ms: record.started_at_ms,
            archived: false,
            recovery_state: RecoveryState::Created,
        },
        runtime: Some(runtime_view(record)),
        agent: None,
        completed_attachment: None,
    }
}

fn runtime_view(record: &RuntimeRecord) -> RuntimeView {
    RuntimeView {
        presentation: Default::default(),
        runtime_epoch: record.runtime_epoch,
        worker_id: record.worker_id,
        worker_pid: record.worker_pid,
        worker_start_time: record.worker_start_time,
        child_pid: record.child_pid,
        child_start_time: record.child_start_time,
        status: yolo_runtime::TerminalRuntimeStatus::Running,
        last_output_sequence: u64::default(),
        exit: None,
        bootstrap_delivery: record.bootstrap_delivery,
        capabilities: None,
        fault_reap: None,
    }
}

fn status(home: &Path, host_pid: u32) -> HostStatus {
    HostStatus {
        protocol_version: yolo_runtime::model::PROTOCOL_VERSION,
        boot_id: TEST_BOOT_ID.into(),
        host_pid,
        controller_generation: TEST_CONTROLLER_GENERATION,
        home: home.to_path_buf(),
        capabilities: Vec::new(),
    }
}
