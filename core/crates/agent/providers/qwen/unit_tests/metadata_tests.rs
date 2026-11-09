use crate::qwen::metadata::{self, TranscriptMetadata};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

const SESSION: &str = "5da2e019-cf3c-4864-a102-e4201ca9afcb";

fn fixture() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = TempDir::new().unwrap();
    let cwd = root.path().join("work");
    fs::create_dir_all(&cwd).unwrap();
    let directory = root.path().join("projects/encoded/chats");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{SESSION}.jsonl"));
    fs::write(&path, "").unwrap();
    (root, cwd, path)
}

#[test]
fn metadata_reads_session_and_canonical_cwd_from_the_first_record() {
    let (_root, cwd, path) = fixture();
    let canonical = std::fs::canonicalize(&cwd).unwrap();
    let value = json!({"sessionId": SESSION, "cwd": canonical});
    let metadata = TranscriptMetadata::from_read_window(
        &path,
        &[value.clone()],
        &[],
        &cwd,
    )
    .unwrap();
    assert_eq!(metadata.session_id, SESSION);
    assert_eq!(metadata.cwd, canonical);
    assert!(metadata::main_transcript_path(&path));
    assert_eq!(metadata::transcript_session_id(&path).as_deref(), Some(SESSION));
}

#[test]
fn filename_or_cwd_mismatch_and_relative_cwd_fail_closed() {
    let (_root, cwd, path) = fixture();
    let canonical = std::fs::canonicalize(&cwd).unwrap();
    let wrong_session = json!({"sessionId": "11111111-1111-4111-8111-111111111111", "cwd": canonical});
    assert!(TranscriptMetadata::from_read_window(&path, &[wrong_session], &[], &cwd).is_none());
    let relative = json!({"sessionId": SESSION, "cwd": "work"});
    assert!(TranscriptMetadata::from_read_window(&path, &[relative], &[], &cwd).is_none());
    let other_cwd = json!({"sessionId": SESSION, "cwd": cwd.parent().unwrap()});
    assert!(TranscriptMetadata::from_read_window(&path, &[other_cwd], &[], &cwd).is_none());
}

#[test]
fn non_transcript_paths_are_not_candidates() {
    let (_root, _cwd, path) = fixture();
    assert!(!metadata::main_transcript_path(&path.with_extension("json")));
    assert!(!metadata::main_transcript_path(
        &path.with_file_name("not-a-session.jsonl")
    ));
    assert!(!metadata::main_transcript_path(
        &path.with_file_name("subagents/agent.jsonl")
    ));
    assert!(metadata::normalized_session_id(SESSION).is_some());
    assert!(metadata::normalized_session_id("not-a-uuid").is_none());
}
