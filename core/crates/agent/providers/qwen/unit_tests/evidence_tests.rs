use crate::qwen::QwenAdapter;
use serde_json::json;
use std::fs;
use std::os::unix::fs::symlink;
use tempfile::TempDir;

const SESSION: &str = "5da2e019-cf3c-4864-a102-e4201ca9afcb";

fn fixture() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let home = TempDir::new().unwrap();
    let cwd = home.path().join("work");
    fs::create_dir_all(&cwd).unwrap();
    let directory = home.path().join("projects/encoded/chats");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{SESSION}.jsonl"));
    let canonical = std::fs::canonicalize(&cwd).unwrap();
    let record = json!({"type": "user", "sessionId": SESSION, "cwd": canonical, "message": {"role": "user", "parts": [{"text": "hello"}]}});
    fs::write(&path, format!("{record}\n")).unwrap();
    (home, cwd, path)
}

#[test]
fn authoritative_evidence_binds_exact_session_and_cwd() {
    let (home, cwd, path) = fixture();
    let evidence = QwenAdapter
        .inspect_authoritative_evidence_with_hook(&path, &cwd, home.path(), || {})
        .unwrap()
        .unwrap();
    assert_eq!(evidence.evidence().provider_session_id, SESSION);
    assert_eq!(evidence.evidence().path, path);
    assert_eq!(evidence.evidence().data_home, home.path());
    assert!(evidence.verify().unwrap());
}

#[test]
fn tampering_or_replacing_the_transcript_fails_verification() {
    let (home, cwd, path) = fixture();
    let mut evidence = QwenAdapter
        .inspect_authoritative_evidence_with_hook(&path, &cwd, home.path(), || {})
        .unwrap()
        .unwrap();
    fs::write(&path, "{}\n").unwrap();
    assert!(!evidence.verify().unwrap());
    assert!(evidence.with_transcript_snapshot(|_, _| Ok(())).unwrap().is_none());

    let (home, cwd, path) = fixture();
    let replacement = path.with_file_name("replacement.jsonl");
    fs::write(&replacement, "{}\n").unwrap();
    fs::rename(&replacement, &path).unwrap();
    assert!(
        QwenAdapter
            .inspect_authoritative_evidence_with_hook(&path, &cwd, home.path(), || {})
            .unwrap()
            .is_none()
    );
}

#[test]
fn symlinked_or_mismatched_transcripts_are_rejected() {
    let (home, cwd, path) = fixture();
    let link = path.with_file_name("link.jsonl");
    symlink(&path, &link).unwrap();
    assert!(
        QwenAdapter
            .inspect_authoritative_evidence_with_hook(&link, &cwd, home.path(), || {})
            .unwrap()
            .is_none()
    );
    let other = home.path().join("work/other.jsonl");
    fs::write(&other, "{}\n").unwrap();
    assert!(
        QwenAdapter
            .inspect_authoritative_evidence_with_hook(&other, &cwd, home.path(), || {})
            .unwrap()
            .is_none()
    );
}
