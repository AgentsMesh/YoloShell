use crate::fake_agent_constants::*;
use crate::packaged_claude_test_support::*;
use crate::packaged_claude_transcript::{Transcript, project_directory_from};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use yolo_runtime::agent::claude::constants as claude;

const WAITING_INDEX: usize = 0;
const RUNNING_INDEX: usize = 1;
const COMPLETION_INDEX: usize = 2;
const COMPLETE_RECORD_COUNT: usize = 3;
const REOPENED_RECORD_COUNT: usize = 2;
const CONFIGURED_PROJECT_DIRECTORY: &str = "-configured-project";
const INVALID_PROJECT_DIRECTORIES: &[&str] = &["", ".", "..", "/absolute", "nested/path"];

#[test]
fn transcript_emits_waiting_running_and_completion_schema_privately() {
    let root = tempfile::tempdir().unwrap();
    let data_home = root.path().join(PACKAGED_RECORD_DATA_HOME_FIELD);
    let cwd = root.path().join(PACKAGED_RECORD_CWD_FIELD);
    std::fs::create_dir(&cwd).unwrap();
    let path = {
        let mut transcript = Transcript::open(&data_home, SESSION_ID, &cwd).unwrap();
        transcript.running(SESSION_ID, &cwd).unwrap();
        transcript.waiting(SESSION_ID, &cwd).unwrap();
        transcript.path().to_path_buf()
    };

    let records = json_lines(&path);
    assert_eq!(records.len(), COMPLETE_RECORD_COUNT);
    assert_common(&records[WAITING_INDEX], SESSION_ID, &cwd);
    assert_common(&records[RUNNING_INDEX], SESSION_ID, &cwd);
    assert_common(&records[COMPLETION_INDEX], SESSION_ID, &cwd);
    assert_waiting(&records[WAITING_INDEX]);
    assert_running(&records[RUNNING_INDEX]);
    assert_completion(&records[COMPLETION_INDEX]);
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & PERMISSION_MODE_MASK,
        PACKAGED_PRIVATE_MODE
    );
    assert_eq!(
        path,
        data_home
            .join(claude::EVIDENCE_DIR)
            .join(PACKAGED_PROJECT_DIRECTORY)
            .join(SESSION_ID)
            .with_extension(yolo_runtime::agent::constants::JSONL_EXTENSION)
    );
}

#[test]
fn reopening_a_transcript_appends_without_truncating_existing_records() {
    let root = tempfile::tempdir().unwrap();
    let data_home = root.path().join(PACKAGED_RECORD_DATA_HOME_FIELD);
    let cwd = root.path();
    let first = Transcript::open(&data_home, SESSION_ID, cwd).unwrap();
    let path = first.path().to_path_buf();
    drop(first);
    let second = Transcript::open(&data_home, SESSION_ID, cwd).unwrap();
    assert_eq!(second.path(), path);
    drop(second);

    let records = json_lines(&path);
    assert_eq!(records.len(), REOPENED_RECORD_COUNT);
    assert!(records.iter().all(|record| {
        record[claude::RECORD_TYPE_KEY] == claude::SYSTEM_RECORD
            && record[claude::SUBTYPE_KEY] == claude::TURN_DURATION_SUBTYPE
    }));
}

#[test]
fn project_directory_override_is_a_single_safe_component() {
    assert_eq!(
        project_directory_from(None).unwrap(),
        std::path::PathBuf::from(PACKAGED_PROJECT_DIRECTORY)
    );
    assert_eq!(
        project_directory_from(Some(OsString::from(CONFIGURED_PROJECT_DIRECTORY))).unwrap(),
        std::path::PathBuf::from(CONFIGURED_PROJECT_DIRECTORY)
    );
    for value in INVALID_PROJECT_DIRECTORIES {
        assert_eq!(
            project_directory_from(Some(OsString::from(value))).unwrap_err(),
            PACKAGED_PROJECT_DIRECTORY_INVALID
        );
    }
}

fn assert_common(record: &Value, session_id: &str, cwd: &std::path::Path) {
    assert_eq!(record[claude::SESSION_ID_KEY], session_id);
    assert_eq!(record[claude::CWD_KEY], json!(cwd));
    assert_eq!(record[PACKAGED_PARENT_UUID_FIELD], Value::Null);
    assert_eq!(record[PACKAGED_SIDECHAIN_FIELD], false);
    assert_eq!(record[PACKAGED_VERSION_FIELD], PACKAGED_TRANSCRIPT_VERSION);
    assert_eq!(record[PACKAGED_TIMESTAMP_FIELD], PACKAGED_TIMESTAMP);
}

fn assert_waiting(record: &Value) {
    assert_eq!(record[claude::RECORD_TYPE_KEY], claude::SYSTEM_RECORD);
    assert_eq!(record[claude::SUBTYPE_KEY], claude::TURN_DURATION_SUBTYPE);
    assert_eq!(record[PACKAGED_DURATION_FIELD], 0);
    assert_eq!(record[PACKAGED_MESSAGE_COUNT_FIELD], 0);
}

fn assert_running(record: &Value) {
    assert_eq!(record[claude::RECORD_TYPE_KEY], claude::USER_RECORD);
    assert_eq!(
        record[claude::MESSAGE_KEY][claude::CONTENT_KEY],
        PACKAGED_REDACTED_USER_CONTENT
    );
}

fn assert_completion(record: &Value) {
    let message = &record[claude::MESSAGE_KEY];
    assert_eq!(record[claude::RECORD_TYPE_KEY], claude::ASSISTANT_RECORD);
    assert_eq!(message[claude::STOP_REASON_KEY], claude::END_TURN_REASON);
    assert_eq!(
        message[claude::CONTENT_KEY][0][claude::CONTENT_TYPE_KEY],
        claude::TEXT_CONTENT
    );
    assert_eq!(
        message[claude::CONTENT_KEY][0][claude::TEXT_VALUE_KEY],
        PACKAGED_REDACTED_ASSISTANT_CONTENT
    );
    assert_eq!(message[claude::CONTEXT_MANAGEMENT_KEY], json!({}));
}
