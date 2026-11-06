fn unspawnable_resume_host(directory: &tempfile::TempDir) -> Host {
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join(RESUME_TEST_MISSING_RUNTIME));
    Host::open(config).unwrap()
}

fn dead_resume_runtime(host: &Host, manifest: &TerminalManifest) -> RuntimeRecord {
    let mut runtime = host_runtime_record(
        host,
        manifest.terminal_id,
        host.paths.worker_socket(manifest.terminal_id).unwrap(),
    );
    runtime.worker_pid = RESUME_TEST_DEAD_WORKER_PID;
    runtime.worker_start_time = RESUME_TEST_PROCESS_START_TIME;
    runtime.child_pid = RESUME_TEST_DEAD_CHILD_PID;
    runtime.child_start_time = RESUME_TEST_PROCESS_START_TIME;
    runtime
}

fn persist_dead_resume_session(host: &Host, manifest: &TerminalManifest) -> RuntimeRecord {
    let runtime = dead_resume_runtime(host, manifest);
    persist_host_session(host, manifest, &runtime);
    #[cfg(target_os = "linux")]
    crate::host::linux_retirement_evidence::publish(host, &runtime);
    runtime
}

fn exact_recreate_request(
    manifest: &TerminalManifest,
    runtime: &RuntimeRecord,
    status: TerminalRuntimeStatus,
) -> ResumeRequest {
    ResumeRequest {
        selection: TerminalLifecycleSelection::selected(manifest.terminal_id, manifest.updated_at_ms, status)
            .with_expected_manifest_sha256(Some(
                crate::model::TerminalManifestDigest::capture(manifest).unwrap(),
            ))
            .with_expected_runtime_epoch(Some(runtime.runtime_epoch)),
        recreate: true,
    }
}

fn shell_resume_launch(host: &Host, manifest: &TerminalManifest) -> WorkerLaunchSpec {
    host.prepare_worker_launch(manifest, shell_command(&manifest.cwd), None, None)
        .unwrap()
}

fn shell_resume_transaction(
    previous_runtime_epoch: Uuid,
    launch: &WorkerLaunchSpec,
) -> ResumeTransaction {
    ResumeTransaction::new(
        previous_runtime_epoch,
        ResumeIntent::Explicit,
        ResumeTarget::Shell {
            binding: ResumeBindingDisposition::Preserve,
        },
        launch,
    )
    .unwrap()
}

fn planned_resume_runtime(
    host: &Host,
    transaction: &ResumeTransaction,
    worker_pid: u32,
    worker_start_time: u64,
    child_pid: u32,
    child_start_time: u64,
    delivery: BootstrapDelivery,
) -> RuntimeRecord {
    let launch = &transaction.launch;
    RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: transaction.terminal_id,
        runtime_epoch: transaction.planned_runtime_epoch,
        worker_id: transaction.worker_id,
        worker_pid,
        worker_start_time,
        child_pid,
        child_start_time,
        boot_id: host.boot_id.clone(),
        socket_path: launch.socket_path.clone(),
        auth_token: launch.auth_token.clone(),
        controller_generation: launch.controller_generation,
        started_at_ms: transaction.created_at_ms,
        command: launch.command.clone(),
        args: launch.args.clone(),
        cwd: launch.cwd.clone(),
        cols: launch.cols,
        rows: launch.rows,
        bootstrap_delivery: delivery,
    }
}

fn spawn_resume_test_child() -> std::process::Child {
    crate::process::spawn_waitable_test_child(
        std::process::Command::new(RESUME_TEST_SLEEP_EXECUTABLE)
            .arg(RESUME_TEST_SLEEP_ARGUMENT)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
    )
}

fn stop_resume_test_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

async fn serve_ready_resume(listener: UnixListener, runtime: RuntimeRecord) {
    let (authentication, _) = listener.accept().await.unwrap();
    let mut authentication = UnixStream::from(authentication);
    let mut authentication = accept_worker_auth(authentication, &runtime).await;
    let (status_stream, _) = listener.accept().await.unwrap();
    let mut status_stream = UnixStream::from(status_stream);
    let mut status_stream = accept_worker_auth(status_stream, &runtime).await;
    let request: RequestEnvelope = status_stream.request().await.unwrap();
    let view = RuntimeView {
        presentation: Default::default(),
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        worker_pid: runtime.worker_pid,
        worker_start_time: runtime.worker_start_time,
        child_pid: runtime.child_pid,
        child_start_time: runtime.child_start_time,
        status: TerminalRuntimeStatus::Running,
        last_output_sequence: 0,
        exit: None,
        bootstrap_delivery: runtime.bootstrap_delivery,
        capabilities: Some(Vec::new()),
        fault_reap: None,
    };
    let response = ResponseEnvelope::success(&request, &view).unwrap();
    status_stream.respond(&response).await.unwrap();
}

async fn serve_resume_auth_once(listener: UnixListener, runtime: RuntimeRecord) {
    let (authentication, _) = listener.accept().await.unwrap();
    let mut authentication = UnixStream::from(authentication);
    let mut authentication = accept_worker_auth(authentication, &runtime).await;
}
