struct ArchiveTestProcess {
    child: Option<std::process::Child>,
    pid: u32,
    start_time: u64,
}

impl ArchiveTestProcess {
    fn spawn() -> Self {
        let child = spawn_resume_test_child();
        let pid = child.id();
        let start_time = process::process_start_time(pid).unwrap();
        Self {
            child: Some(child),
            pid,
            start_time,
        }
    }

    fn identity(&self) -> (u32, u64) {
        (self.pid, self.start_time)
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            stop_resume_test_child(&mut child);
        }
    }
}

impl Drop for ArchiveTestProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

fn bind_archive_processes(
    record: &mut RuntimeRecord,
    worker: &ArchiveTestProcess,
    child: &ArchiveTestProcess,
) {
    (record.worker_pid, record.worker_start_time) = worker.identity();
    (record.child_pid, record.child_start_time) = child.identity();
}

fn dead_archive_record(host: &Host, terminal_id: Uuid, socket_path: PathBuf) -> RuntimeRecord {
    let mut record = host_runtime_record(host, terminal_id, socket_path);
    record.worker_pid = RESUME_TEST_DEAD_WORKER_PID;
    record.worker_start_time = RESUME_TEST_PROCESS_START_TIME;
    record.child_pid = RESUME_TEST_DEAD_CHILD_PID;
    record.child_start_time = RESUME_TEST_PROCESS_START_TIME;
    record
}

fn archive_exit_for(runtime_epoch: Uuid) -> ExitRecord {
    ExitRecord {
        runtime_epoch,
        code: None,
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: u64::default(),
    }
}

fn persist_archive_runtime(host: &Host, record: &RuntimeRecord) {
    host.paths.ensure_session(record.terminal_id).unwrap();
    atomic_write_json(&host.paths.runtime_record(record.terminal_id), record).unwrap();
    host.secondary_custody.authorize_rpc_test(record);
}

fn persist_archive_exit(host: &Host, record: &RuntimeRecord, runtime_epoch: Uuid) -> ExitRecord {
    let exit = archive_exit_for(runtime_epoch);
    atomic_write_json(&host.paths.exit_record(record.terminal_id), &exit).unwrap();
    #[cfg(target_os = "linux")]
    if runtime_epoch == record.runtime_epoch {
        crate::host::linux_retirement_evidence::publish(host, record);
    }
    exit
}

fn archive_exited_view(record: &RuntimeRecord, exit: ExitRecord) -> RuntimeView {
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
        capabilities: Some(Vec::new()),
        fault_reap: None,
    }
}

async fn accept_archive_request(
    listener: &UnixListener,
    record: &RuntimeRecord,
) -> (yolo_runtime_rpc_testing::Peer, RequestEnvelope) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    accept_archive_worker_auth(&mut stream, record).await;
    let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
    let request = stream.request().await.unwrap();
    (stream, request)
}

async fn accept_archive_worker_auth(stream: &mut UnixStream, record: &RuntimeRecord) {
    let auth: RequestEnvelope = read_frame(stream).await.unwrap();
    let hello = crate::model::WorkerHello {
        runtime: record.clone(),
        capabilities: Vec::new(),
    };
    let response = ResponseEnvelope::success(&auth, &hello).unwrap();
    write_frame(stream, &response).await.unwrap();
}
