use crate::host::tests::*;
const RESUME_CONVERGENCE_CORRUPT_JSON: &[u8] = b"{invalid-resume-json";
const RESUME_CONVERGENCE_FIELD_SUFFIX: &str = "-changed-after-plan";
const RESUME_CONVERGENCE_INVALID_START_TIME: u64 = 0;

fn live_resume_runtime(
    host: &Host,
    transaction: &ResumeTransaction,
    delivery: BootstrapDelivery,
) -> RuntimeRecord {
    let worker_pid = std::process::id();
    let child_pid = u32::try_from(unsafe { libc::getppid() }).unwrap();
    planned_resume_runtime(
        host,
        transaction,
        worker_pid,
        process::process_start_time(worker_pid).unwrap(),
        child_pid,
        process::process_start_time(child_pid).unwrap(),
        delivery,
    )
}

fn corrupt_resume_json(path: &Path) {
    write_private_bytes(path, RESUME_CONVERGENCE_CORRUPT_JSON);
}

fn assert_resume_failure_retains<T: std::fmt::Debug>(
    host: &Host,
    expected: &ResumeTransaction,
    result: std::result::Result<T, SessionResumeFailure>,
) {
    assert_resume_transaction_retained(host, expected, result);
    assert_eq!(
        host.load_durable_manifest(expected.terminal_id)
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );
}

fn assert_resume_transaction_retained<T: std::fmt::Debug>(
    host: &Host,
    expected: &ResumeTransaction,
    result: std::result::Result<T, SessionResumeFailure>,
) {
    let error = result.unwrap_err().into_explicit_error();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(!error.retryable);
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(expected.terminal_id))
            .unwrap(),
        *expected
    );
}

pub(super) fn resume_manifest_identity(host: &Host, terminal_id: Uuid) -> crate::paths::identity::FileIdentity {
    host.namespace
        .file(&host.paths.manifest(terminal_id))
        .unwrap()
        .inspect(crate::paths::MAX_DURABLE_JSON_BYTES)
        .unwrap()
        .unwrap()
}

fn assert_resume_failure_retains_corruption<T: std::fmt::Debug>(
    host: &Host,
    expected: &ResumeTransaction,
    identity: crate::paths::identity::FileIdentity,
    result: std::result::Result<T, SessionResumeFailure>,
) {
    assert_resume_transaction_retained(host, expected, result);
    assert_eq!(
        resume_manifest_identity(host, expected.terminal_id),
        identity
    );
    assert_eq!(
        std::fs::read(host.paths.manifest(expected.terminal_id)).unwrap(),
        RESUME_CONVERGENCE_CORRUPT_JSON
    );
    assert!(host.load_durable_manifest(expected.terminal_id).is_err());
}

fn assert_provider_completion_pending_retains<T: std::fmt::Debug>(
    host: &Host,
    expected: &ResumeTransaction,
    result: std::result::Result<T, SessionResumeFailure>,
) {
    let error = result.unwrap_err().into_explicit_error();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(error.retryable);
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(expected.terminal_id))
            .unwrap(),
        *expected
    );
    assert_eq!(
        host.load_durable_manifest(expected.terminal_id)
            .unwrap()
            .recovery_state,
        RecoveryState::Restoring
    );
    assert!(host.paths.worker_launch(expected.terminal_id).exists());
}

async fn serve_auth_after_runtime_corruption(
    listener: UnixListener,
    runtime: RuntimeRecord,
    runtime_path: PathBuf,
) {
    let (mut stream, _) = listener.accept().await.unwrap();
    let request: RequestEnvelope = read_frame(&mut stream).await.unwrap();
    corrupt_resume_json(&runtime_path);
    let response = ResponseEnvelope::success(&request, &test_worker_hello(&runtime)).unwrap();
    write_frame(&mut stream, &response).await.unwrap();
}

async fn serve_replayed_resume_ready(listener: UnixListener, runtime: RuntimeRecord) {
    let (replay_stream, _) = listener.accept().await.unwrap();
    let mut replay_stream = UnixStream::from(replay_stream);
    let mut replay_stream = accept_worker_auth(replay_stream, &runtime).await;
    serve_ready_resume(listener, runtime).await;
}

fn write_unexpected_resume_binding_marker(host: &Host, manifest: &TerminalManifest) {
    let transaction = binding_clear_transaction(host, manifest, None);
    let marker = binding_clear_marker(&transaction, ResumeBindingClearPhase::Pending).unwrap();
    host.write_namespace_json(
        &host.paths.resume_binding_clear(manifest.terminal_id),
        &marker,
    )
    .unwrap();
}
