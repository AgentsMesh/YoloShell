use crate::fake_agent_constants::*;
use crate::packaged_claude_record::{self, InvocationRecord};
use crate::packaged_claude_test_support::*;
use serde_json::Value;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;

#[test]
fn fresh_and_resume_records_are_private_exact_and_create_new() {
    let directory = tempfile::tempdir().unwrap();
    let cwd = directory.path();
    let data_home = cwd.join(PACKAGED_RECORD_DATA_HOME_FIELD);
    let transcript = cwd.join(TRANSCRIPT_FILE_NAME);
    let arguments = strings(&[
        yolo_runtime::agent::claude::constants::RESUME_ARGUMENT,
        SESSION_ID,
    ]);
    let fresh = record(
        PACKAGED_FRESH_MODE,
        &arguments,
        cwd,
        &data_home,
        &transcript,
    );
    packaged_claude_record::write_at(directory.path(), RUNTIME_EPOCH, fresh).unwrap();
    let fresh_path = directory.path().join(PACKAGED_FRESH_RECORD);
    assert_record(
        &fresh_path,
        PACKAGED_FRESH_MODE,
        cwd,
        &data_home,
        &transcript,
    );

    let duplicate = record(
        PACKAGED_FRESH_MODE,
        &arguments,
        cwd,
        &data_home,
        &transcript,
    );
    let error =
        packaged_claude_record::write_at(directory.path(), RUNTIME_EPOCH, duplicate).unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind),
        Some(ErrorKind::AlreadyExists)
    );

    let resume = record(
        PACKAGED_RESUME_MODE,
        &arguments,
        cwd,
        &data_home,
        &transcript,
    );
    packaged_claude_record::write_at(directory.path(), RUNTIME_EPOCH, resume).unwrap();
    assert_record(
        &directory.path().join(PACKAGED_RESUME_RECORD),
        PACKAGED_RESUME_MODE,
        cwd,
        &data_home,
        &transcript,
    );
}

fn record<'a>(
    mode: &'a str,
    arguments: &'a [String],
    cwd: &'a std::path::Path,
    data_home: &'a std::path::Path,
    transcript: &'a std::path::Path,
) -> InvocationRecord<'a> {
    InvocationRecord {
        mode,
        session_id: SESSION_ID,
        arguments,
        hook_count: PACKAGED_HOOK_EVENTS.len(),
        cwd,
        data_home,
        transcript,
    }
}

fn assert_record(
    path: &std::path::Path,
    mode: &str,
    cwd: &std::path::Path,
    data_home: &std::path::Path,
    transcript: &std::path::Path,
) {
    let contents = std::fs::read(path).unwrap();
    assert_eq!(contents.last(), Some(&INVOCATION_LINE_ENDING));
    let value: Value = serde_json::from_slice(&contents).unwrap();
    assert_eq!(
        value[PACKAGED_RECORD_SCHEMA_FIELD],
        PACKAGED_RECORD_SCHEMA_VERSION
    );
    assert_eq!(value[PACKAGED_RECORD_MODE_FIELD], mode);
    assert_eq!(value[PACKAGED_RECORD_SESSION_FIELD], SESSION_ID);
    assert_eq!(value[PACKAGED_RECORD_RUNTIME_FIELD], RUNTIME_EPOCH);
    assert_eq!(
        value[PACKAGED_RECORD_HOOK_COUNT_FIELD],
        PACKAGED_HOOK_EVENTS.len()
    );
    assert_eq!(
        value[PACKAGED_RECORD_CWD_FIELD],
        cwd.to_string_lossy().as_ref()
    );
    assert_eq!(
        value[PACKAGED_RECORD_DATA_HOME_FIELD],
        data_home.to_string_lossy().as_ref()
    );
    assert_eq!(
        value[PACKAGED_RECORD_TRANSCRIPT_FIELD],
        transcript.to_string_lossy().as_ref()
    );
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & PERMISSION_MODE_MASK,
        PACKAGED_PRIVATE_MODE
    );
}
