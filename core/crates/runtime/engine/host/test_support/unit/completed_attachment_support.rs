const COMPLETED_TEST_SOCKET_NAME: &str = "completed-active.sock";
const COMPLETED_TEST_EXIT_CODE: i32 = 7;
const COMPLETED_TEST_OBSERVED_AT_MS: i64 = 11;
const COMPLETED_TEST_OUTPUT_SEQUENCE: u64 = 13;
const COMPLETED_TEST_SNAPSHOT: &[u8] = b"completed-screen";
const COMPLETED_TEST_WORKER_NOT_RUNNING: &str = "completed test Worker rejected control";

struct ActiveCompletedFixture {
    host: Host,
    record: RuntimeRecord,
    manifest: TerminalManifest,
    runtime: RuntimeView,
    listener: UnixListener,
    _directory: HostTestDirectory,
}

fn completed_test_manifest(host: &Host, terminal_id: Uuid) -> TerminalManifest {
    let mut manifest = fixture_manifest(host.paths.home());
    manifest.terminal_id = terminal_id;
    manifest
}

fn completed_test_exit(record: &RuntimeRecord, reason: ExitReason) -> ExitRecord {
    ExitRecord {
        runtime_epoch: record.runtime_epoch,
        code: Some(COMPLETED_TEST_EXIT_CODE),
        signal: None,
        reason,
        observed_at_ms: COMPLETED_TEST_OBSERVED_AT_MS,
        last_output_sequence: COMPLETED_TEST_OUTPUT_SEQUENCE,
    }
}

fn completed_test_runtime(
    record: &RuntimeRecord,
    exit: ExitRecord,
    capabilities: Option<Vec<String>>,
) -> RuntimeView {
    RuntimeView {
        presentation: Default::default(),
        runtime_epoch: record.runtime_epoch,
        worker_id: record.worker_id,
        worker_pid: record.worker_pid,
        worker_start_time: record.worker_start_time,
        child_pid: record.child_pid,
        child_start_time: record.child_start_time,
        status: TerminalRuntimeStatus::Exited,
        last_output_sequence: exit.last_output_sequence,
        exit: Some(exit),
        bootstrap_delivery: record.bootstrap_delivery,
        capabilities,
        fault_reap: None,
    }
}

fn active_completed_fixture() -> ActiveCompletedFixture {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(COMPLETED_TEST_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let terminal_id = Uuid::new_v4();
    host.paths.ensure_session(terminal_id).unwrap();
    let record = host_runtime_record(&host, terminal_id, socket);
    let exit = completed_test_exit(&record, ExitReason::Natural);
    atomic_write_json(&host.paths.runtime_record(terminal_id), &record).unwrap();
    atomic_write_json(&host.paths.exit_record(terminal_id), &exit).unwrap();
    ActiveCompletedFixture {
        manifest: completed_test_manifest(&host, terminal_id),
        runtime: completed_test_runtime(&record, exit, Some(Vec::new())),
        _directory: HostTestDirectory::capture(directory, &host.paths),
        host,
        record,
        listener,
    }
}

async fn accept_completed_worker_auth(
    mut stream: UnixStream,
    record: &RuntimeRecord,
) -> yolo_runtime_rpc_testing::Peer {
    let auth: RequestEnvelope = read_frame(&mut stream).await.unwrap();
    let hello = crate::model::WorkerHello {
        runtime: record.clone(),
        capabilities: Vec::new(),
    };
    write_frame(
        &mut stream,
        &ResponseEnvelope::success(&auth, &hello).unwrap(),
    )
    .await
    .unwrap();
    yolo_runtime_rpc_testing::Peer::new(stream)
}

async fn serve_completed_worker_status(
    listener: UnixListener,
    record: RuntimeRecord,
    runtime: RuntimeView,
) {
    const STATUS_REQUEST_COUNT: usize = 1;

    serve_completed_worker_status_count(listener, record, runtime, STATUS_REQUEST_COUNT).await;
}

async fn serve_completed_worker_status_count(
    listener: UnixListener,
    record: RuntimeRecord,
    runtime: RuntimeView,
    count: usize,
) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = accept_completed_worker_auth(UnixStream::from(stream), &record).await;
    for _ in 0..count {
        let status = stream.request().await.unwrap();
        assert_eq!(status.method, WORKER_STATUS_METHOD);
        stream
            .respond(&ResponseEnvelope::success(&status, &runtime).unwrap())
            .await
            .unwrap();
    }
}

async fn serve_completed_worker_rejections(
    listener: UnixListener,
    record: RuntimeRecord,
    methods: &'static [&'static str],
) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = accept_completed_worker_auth(UnixStream::from(stream), &record).await;
    for expected_method in methods {
        let request = stream.request().await.unwrap();
        assert_eq!(request.method, *expected_method);
        let error = ErrorBody::new(
            crate::lease::constants::code::SESSION_NOT_RUNNING,
            COMPLETED_TEST_WORKER_NOT_RUNNING,
            false,
        );
        stream
            .respond(&ResponseEnvelope::failure(&request, error))
            .await
            .unwrap();
    }
}

async fn await_completed_worker(server: tokio::task::JoinHandle<()>) {
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, server)
        .await
        .unwrap()
        .unwrap();
}

fn completed_test_snapshot(exit: &ExitRecord) -> crate::model::TerminalSnapshot {
    crate::model::TerminalSnapshot {
        runtime_epoch: exit.runtime_epoch,
        barrier_sequence: exit.last_output_sequence,
        cols: TEST_TERMINAL_COLUMNS,
        rows: TEST_TERMINAL_ROWS,
        data: COMPLETED_TEST_SNAPSHOT.to_vec(),
    }
}
