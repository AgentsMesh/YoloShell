use crate::host::tests::*;
pub(super) fn completion_provider_binding(directory: &tempfile::TempDir, terminal_id: Uuid) -> AgentBinding {
    let root = directory.path().canonicalize().unwrap();
    let cwd = root.join(COMPLETION_PROVIDER_CWD_DIRECTORY);
    let data_home = root.join(COMPLETION_PROVIDER_HOME_DIRECTORY);
    let evidence_directory = data_home
        .join(crate::agent::codex::constants::EVIDENCE_DIR)
        .join(COMPLETION_PROVIDER_EVIDENCE_DIRECTORY);
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(&evidence_directory).unwrap();
    let evidence = evidence_directory.join(format!(
        "{}{COMPLETION_PROVIDER_ROLLOUT_STAMP}-{COMPLETION_PROVIDER_SESSION_ID}.{}",
        crate::agent::codex::constants::ROLLOUT_FILE_PREFIX,
        crate::agent::constants::JSONL_EXTENSION,
    ));
    write_completion_provider_evidence(&evidence, &cwd);
    AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: terminal_id,
        kind: crate::agent::AgentKind::Codex,
        provider_session_id: COMPLETION_PROVIDER_SESSION_ID.into(),
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        cwd: cwd.to_string_lossy().into_owned(),
        evidence_path: evidence.to_string_lossy().into_owned(),
        data_home: data_home.to_string_lossy().into_owned(),
        process_pid: Some(COMPLETION_PROVIDER_PREVIOUS_PID),
        process_start_time: Some(COMPLETION_PROVIDER_PREVIOUS_START_TIME),
        state: crate::agent::AgentState::Running,
        recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
        detected_at_ms: COMPLETION_PROVIDER_INITIAL_TIMESTAMP,
        updated_at_ms: COMPLETION_PROVIDER_INITIAL_TIMESTAMP,
    }
}

fn write_completion_provider_evidence(path: &Path, cwd: &Path) {
    use crate::agent::codex::constants as codex;
    let metadata = serde_json::json!({
        codex::RECORD_TYPE_KEY: codex::SESSION_META_TYPE,
        codex::PAYLOAD_KEY: {
            codex::SESSION_ID_KEY: COMPLETION_PROVIDER_SESSION_ID,
            codex::CWD_KEY: cwd,
            codex::SOURCE_KEY: codex::CLI_SOURCE,
        }
    });
    let running = serde_json::json!({
        codex::RECORD_TYPE_KEY: codex::EVENT_MESSAGE_TYPE,
        codex::PAYLOAD_KEY: { codex::EVENT_KIND_KEY: codex::TASK_STARTED_EVENT }
    });
    let ending = char::from(codex::JSON_LINE_ENDING_BYTE);
    std::fs::write(path, format!("{metadata}{ending}{running}{ending}")).unwrap();
}

async fn finish_provider_auth_only(
    host: &Host,
    manifest: TerminalManifest,
    transaction: &ResumeTransaction,
    runtime: &RuntimeRecord,
) -> std::result::Result<TerminalView, SessionResumeFailure> {
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_resume_auth_once(listener, runtime.clone()));
    let result = host.finish_planned_resume(manifest, transaction).await;
    settle_provider_server(server).await;
    drop(publication);
    result
}

async fn finish_provider_ready(
    host: &Host,
    manifest: TerminalManifest,
    transaction: &ResumeTransaction,
    runtime: &RuntimeRecord,
) -> std::result::Result<TerminalView, SessionResumeFailure> {
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_ready_resume(listener, runtime.clone()));
    let result = host.finish_planned_resume(manifest, transaction).await;
    settle_provider_server(server).await;
    drop(publication);
    result
}

async fn settle_provider_server(server: tokio::task::JoinHandle<()>) {
    tokio::time::timeout(COMPLETION_PROVIDER_SERVER_TIMEOUT, server)
        .await
        .unwrap()
        .unwrap();
}
