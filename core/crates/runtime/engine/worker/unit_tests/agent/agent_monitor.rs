
fn monitor_runtime() -> (
    std::sync::Arc<crate::agent::AgentRegistry>,
    std::sync::Arc<yolo_agent_providers::ProviderRuntimeRegistry>,
) {
    let provider_runtime = std::sync::Arc::new(yolo_agent_providers::builtin_provider_runtime());
    let registry = std::sync::Arc::new(crate::agent::agent_registry_from_runtime(
        &provider_runtime,
    ));
    (registry, provider_runtime)
}

use crate::worker::tests::*;
const TEST_CODEX_ROLLOUT_LABEL: &str = "test-";
const TEST_CODEX_ROLLOUT_DATE_DIRECTORY: &str = "2026/08/05";

pub(super) fn binding(fixture: &Fixture, state: crate::agent::AgentState) -> AgentBinding {
    let data_home = fixture.state.paths.home().join("codex-home");
    let provider_session_id = fixture.terminal_id().to_string();
    let evidence = data_home
        .join(crate::agent::codex::constants::EVIDENCE_DIR)
        .join(TEST_CODEX_ROLLOUT_DATE_DIRECTORY)
        .join(format!(
            "{}{}{provider_session_id}.{}",
            crate::agent::codex::constants::ROLLOUT_FILE_PREFIX,
            TEST_CODEX_ROLLOUT_LABEL,
            crate::agent::constants::JSONL_EXTENSION,
        ));
    std::fs::create_dir_all(evidence.parent().unwrap()).unwrap();
    let metadata = serde_json::json!({
        (crate::agent::codex::constants::RECORD_TYPE_KEY):
            crate::agent::codex::constants::SESSION_META_TYPE,
        (crate::agent::codex::constants::PAYLOAD_KEY): {
            (crate::agent::codex::constants::SESSION_ID_KEY): &provider_session_id,
            (crate::agent::codex::constants::CWD_KEY): fixture.state.paths.home(),
            (crate::agent::codex::constants::SOURCE_KEY):
                crate::agent::codex::constants::CLI_SOURCE,
        },
    });
    std::fs::write(
        &evidence,
        format!("{metadata}{}", crate::agent::constants::COMMAND_LINE_ENDING),
    )
    .unwrap();
    AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: fixture.terminal_id(),
        kind: crate::agent::AgentKind::Codex,
        provider_session_id,
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        cwd: fixture.state.paths.home().display().to_string(),
        evidence_path: evidence.display().to_string(),
        data_home: data_home.display().to_string(),
        process_pid: Some(2),
        process_start_time: Some(2),
        state,
        recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
        detected_at_ms: 1,
        updated_at_ms: 2,
    }
}

fn binding_context(fixture: &Fixture) -> DetectionContext {
    DetectionContext::new(
        fixture.terminal_id(),
        fixture.state.paths.home().to_path_buf(),
        [(
            crate::agent::AgentKind::Codex,
            fixture.state.paths.home().join("codex-home"),
        )],
    )
}

#[tokio::test]
async fn agent_detection_persistence_emits_only_canonical_changes() {
    let fixture = Fixture::new();
    let store = AgentBindingStore::new(fixture.state.paths.agent_binding(fixture.terminal_id()));
    let mut events = fixture.state.events.subscribe();
    persist_agent_detection(
        &fixture.state,
        &store,
        DetectionResult::BindingChanged(binding(&fixture, crate::agent::AgentState::Running)),
    )
    .await
    .unwrap();
    assert!(
        matches!(events.try_recv(), Ok(WorkerEvent::Agent { event_type, .. }) if event_type == event::AGENT_BINDING_CHANGED)
    );
    persist_agent_detection(
        &fixture.state,
        &store,
        DetectionResult::StatusChanged(binding(
            &fixture,
            crate::agent::AgentState::WaitingForInput,
        )),
    )
    .await
    .unwrap();
    assert!(events.try_recv().is_err());
    persist_agent_detection(&fixture.state, &store, DetectionResult::Unchanged)
        .await
        .unwrap();
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn agent_persistence_failure_is_reported_for_monitor_restart() {
    let fixture = Fixture::new();
    let invalid_path = fixture.state.paths.home().join("binding-directory");
    std::fs::create_dir_all(&invalid_path).unwrap();
    let store = AgentBindingStore::new(invalid_path);
    let mut events = fixture.state.events.subscribe();
    let result = persist_agent_detection(
        &fixture.state,
        &store,
        DetectionResult::BindingChanged(binding(&fixture, crate::agent::AgentState::Running)),
    )
    .await;
    assert!(result.is_err());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn provider_title_tracks_the_active_agent_run() {
    let fixture = Fixture::new();
    let live = binding(&fixture, crate::agent::AgentState::Running);
    let title = "finished conversation";
    std::fs::write(
        std::path::Path::new(&live.data_home)
            .join(crate::agent::codex::constants::SESSION_INDEX_FILE),
        format!(
            "{{\"{}\":\"{}\",\"{}\":\"{title}\"}}\n",
            crate::agent::codex::constants::SESSION_INDEX_ID_KEY,
            live.provider_session_id,
            crate::agent::codex::constants::SESSION_INDEX_TITLE_KEY,
        ),
    )
    .unwrap();

    refresh_agent_title(&fixture.state, Some(live))
        .await
        .unwrap();
    assert_eq!(
        fixture.state.status().presentation.title.as_deref(),
        Some(title)
    );

    refresh_agent_title(
        &fixture.state,
        Some(binding(&fixture, crate::agent::AgentState::Exited)),
    )
    .await
    .unwrap();
    assert_eq!(fixture.state.status().presentation.title, None);

    refresh_agent_title(&fixture.state, None).await.unwrap();
    assert_eq!(fixture.state.status().presentation.title, None);
}

#[test]
fn retained_manifest_check_distinguishes_present_and_missing() {
    let fixture = Fixture::new();
    let manifest = fixture.state.paths.manifest(fixture.terminal_id());
    std::fs::remove_file(&manifest).unwrap();
    assert!(fixture.state.namespace.file_is_missing(&manifest).unwrap());
    std::fs::write(&manifest, b"manifest").unwrap();
    assert!(!fixture.state.namespace.file_is_missing(&manifest).unwrap());
}

#[test]
fn agent_monitor_rebuild_reloads_store_after_a_failed_construction() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let binding_path = fixture.state.paths.agent_binding(fixture.terminal_id());
    std::fs::write(&binding_path, b"not-json").unwrap();
    std::fs::set_permissions(
        &binding_path,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    let (registry, provider_runtime) = monitor_runtime();
    let config = AgentMonitorConfig {
        registry,
        provider_runtime,
        context: binding_context(&fixture),
        binding_path: binding_path.clone(),
    };
    assert!(build_agent_monitor(&config).is_err());
    let expected = binding(&fixture, crate::agent::AgentState::Running);
    AgentBindingStore::new(binding_path)
        .save(&expected)
        .unwrap();
    let (_, rebuilt_store) = build_agent_monitor(&config).unwrap();
    assert_exact_binding_and_fail_closed_recovery(&rebuilt_store, &config.context, &expected);
}


include!("agent_monitor_termination.rs");
