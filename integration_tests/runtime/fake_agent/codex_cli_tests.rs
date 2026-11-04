use super::{codex_cli, fake_agent_constants as constants, provider::FakeAgentProvider};
use crate::packaged_claude_test_support::*;
use serde_json::json;
use std::path::Path;

#[test]
fn codex_version_probe_matches_the_pinned_provider_contract() {
    let output = codex_cli::version_output(
        FakeAgentProvider::Codex,
        &[constants::CODEX_VERSION_ARGUMENT.to_owned()],
    );
    assert_eq!(
        output.as_deref(),
        Some(constants::CODEX_EXPECTED_VERSION_OUTPUT)
    );
    assert!(codex_cli::version_output(FakeAgentProvider::Claude, &[]).is_none());
    assert!(codex_cli::version_output(FakeAgentProvider::Codex, &[]).is_none());
}

#[test]
fn session_start_serializes_the_actual_pinned_raw_dto() {
    let payload = codex_cli::session_start_payload_for_test(
        constants::CODEX_TEST_SESSION_ID,
        Path::new(constants::CODEX_TEST_TRANSCRIPT),
        Path::new(constants::CODEX_TEST_CWD),
        constants::CODEX_SESSION_RESUME_SOURCE,
    );
    assert_eq!(
        payload,
        json!({
            constants::CODEX_HOOK_EVENT_NAME: constants::CODEX_SESSION_START_EVENT,
            constants::CODEX_HOOK_SESSION_ID: constants::CODEX_TEST_SESSION_ID,
            constants::CODEX_HOOK_TRANSCRIPT_PATH: constants::CODEX_TEST_TRANSCRIPT,
            constants::CODEX_HOOK_CWD: constants::CODEX_TEST_CWD,
            constants::CODEX_HOOK_MODEL: constants::CODEX_MODEL,
            constants::CODEX_HOOK_PERMISSION_MODE: constants::CODEX_PERMISSION_DEFAULT,
            constants::CODEX_HOOK_SOURCE: constants::CODEX_SESSION_RESUME_SOURCE,
        })
    );
    let _: yolo_runtime::codex_hooks::CodexHookInput = serde_json::from_value(payload).unwrap();
}

#[test]
fn native_codex_executes_hooks_json_without_runtime_launch_markers() {
    let home = tempfile::tempdir().unwrap();
    let helper = executable(home.path(), HELPER_FILE_NAME, SUCCESS_SCRIPT);
    let capture = home.path().join(CAPTURE_FILE_NAME);
    let hook = command(&helper, Some(&capture));
    let configuration = yolo_hook_installation_catalog::CODEX.file_name;
    write_json(
        &home.path().join(configuration),
        &json!({
            (constants::PACKAGED_SETTINGS_HOOKS_FIELD): {
                (constants::CODEX_SESSION_START_EVENT): hook_event(&hook.command),
            }
        }),
    );
    codex_cli::deliver_session_start(
        FakeAgentProvider::Codex,
        home.path(),
        &[],
        SESSION_ID,
        &home.path().join(TRANSCRIPT_FILE_NAME),
        home.path(),
    )
    .unwrap();
    let payload: serde_json::Value =
        serde_json::from_slice(&std::fs::read(capture).unwrap()).unwrap();
    assert_eq!(
        payload[constants::CODEX_HOOK_SOURCE],
        constants::CODEX_SESSION_STARTUP_SOURCE
    );
    assert_eq!(payload[constants::CODEX_HOOK_SESSION_ID], SESSION_ID);
}

#[test]
fn missing_hooks_do_not_require_any_yolo_injection() {
    let home = tempfile::tempdir().unwrap();
    codex_cli::deliver_session_start(
        FakeAgentProvider::Codex,
        home.path(),
        &[],
        SESSION_ID,
        &home.path().join(TRANSCRIPT_FILE_NAME),
        home.path(),
    )
    .unwrap();
}
