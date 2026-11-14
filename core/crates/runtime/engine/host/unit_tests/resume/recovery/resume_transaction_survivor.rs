use crate::host::tests::*;
use super::*;

pub(super) const SURVIVOR_RUNTIME_RACE_OFFSET: i64 = 1;
async fn accept_adopting_resume_auth(
    stream: &mut UnixStream,
    mut runtime: RuntimeRecord,
    runtime_path: &Path,
) -> RuntimeRecord {
    let request: RequestEnvelope = read_frame(stream).await.unwrap();
    let auth: crate::worker::WorkerAuth = request.decode_payload().unwrap();
    assert_eq!(auth.token, runtime.auth_token);
    runtime.controller_generation = auth.controller_generation;
    atomic_write_json(runtime_path, &runtime).unwrap();
    let response = ResponseEnvelope::success(&request, &test_worker_hello(&runtime)).unwrap();
    write_frame(stream, &response).await.unwrap();
    runtime
}

async fn serve_adopting_survivor(
    listener: UnixListener,
    runtime: RuntimeRecord,
    runtime_path: PathBuf,
) {
    let (authentication, _) = listener.accept().await.unwrap();
    let mut authentication = UnixStream::from(authentication);
    let runtime = accept_adopting_resume_auth(&mut authentication, runtime, &runtime_path).await;
    let (status_stream, _) = listener.accept().await.unwrap();
    let mut status_stream = UnixStream::from(status_stream);
    let mut status_stream = accept_worker_auth(status_stream, &runtime).await;
    let request: RequestEnvelope = status_stream.request().await.unwrap();
    assert_eq!(request.method, WORKER_STATUS_METHOD);
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
        bootstrap_delivery: BootstrapDelivery::Survivor,
        capabilities: Some(Vec::new()),
        fault_reap: None,
    };
    status_stream
        .respond(&ResponseEnvelope::success(&request, &view).unwrap())
        .await
        .unwrap();
}

pub(super) async fn resume_survivor_explicit(
    host: &Host,
    terminal_id: Uuid,
) -> std::result::Result<TerminalView, SessionResumeFailure> {
    host.resume_session_locked(
        terminal_id,
        ResumeSelection::Preserve(ResumeIntent::Explicit),
    )
    .await
}

#[tokio::test]
async fn survivor_resume_adopts_restarted_host_and_preserves_authority() {
    let directory = tempdir().unwrap();
    let first = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&first, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let mut child = spawn_resume_test_child();
    let runtime = planned_resume_runtime(
        &first,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        child.id(),
        process::process_start_time(child.id()).unwrap(),
        BootstrapDelivery::Survivor,
    );
    let previous_generation = first.controller_generation;
    persist_survivor_resume(&first, &manifest, &transaction, &runtime);
    drop(first);

    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    assert!(host.controller_generation > previous_generation);
    host.secondary_custody.authorize_rpc_test(&runtime);
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_adopting_survivor(
        listener,
        runtime,
        host.paths.runtime_record(manifest.terminal_id),
    ));
    let view = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Explicit),
        )
        .await
        .unwrap();
    server.await.unwrap();
    drop(publication);

    assert_eq!(view.manifest.recovery_state, RecoveryState::Uncertain);
    assert_eq!(
        view.runtime.as_ref().unwrap().status,
        TerminalRuntimeStatus::Running
    );
    assert_eq!(
        view.runtime.as_ref().unwrap().capabilities,
        Some(Vec::new())
    );
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(manifest.terminal_id))
            .unwrap()
            .controller_generation,
        host.controller_generation
    );
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        transaction
    );
    assert!(host.paths.worker_launch(manifest.terminal_id).exists());
    assert!(host.paths.terminate_tombstone(manifest.terminal_id).exists());
    assert!(!host.resume_transaction_needs_reconcile(&view.manifest));
    for selection in [
        ResumeSelection::Preserve(ResumeIntent::Explicit),
        ResumeSelection::Preserve(ResumeIntent::Automatic),
    ] {
        let error = host
            .resume_session_locked(manifest.terminal_id, selection)
            .await
            .unwrap_err()
            .into_explicit_error();
        assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    }
    stop_resume_test_child(&mut child);
}

#[tokio::test]
async fn provider_resume_survivor_retains_unproven_bootstrap_transaction() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let (_, _, transaction) = provider_resume_fixture(&host, &manifest);
    assert!(transaction.launch.initial_input.is_some());
    let mut child = spawn_resume_test_child();
    let runtime = planned_resume_runtime(
        &host,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        child.id(),
        process::process_start_time(child.id()).unwrap(),
        BootstrapDelivery::Survivor,
    );
    persist_survivor_resume(&host, &manifest, &transaction, &runtime);
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_ready_resume(listener, runtime));

    let view = resume_survivor_explicit(&host, manifest.terminal_id)
        .await
        .unwrap();
    server.await.unwrap();
    drop(publication);
    stop_resume_test_child(&mut child);
    assert_eq!(view.manifest.recovery_state, RecoveryState::Uncertain);
    assert_eq!(
        view.runtime.unwrap().bootstrap_delivery,
        BootstrapDelivery::Survivor
    );
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        transaction
    );
}
