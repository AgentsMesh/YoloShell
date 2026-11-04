use super::test_support::{Fixture, OTHER_SESSION_ID, SESSION_ID};
use super::{AuthoritativeCodexEvidence, CodexAdapter, constants};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const DISPLACED_FILE: &str = "displaced-rollout";
const DISPLACED_DIRECTORY: &str = "displaced-directory";
const MISSING_FILE: &str = "missing-rollout.jsonl";
const APPEND_EVENT: &[u8] = b"{\"type\":\"event_msg\",\"payload\":{}}\n";

fn capture(fixture: &Fixture, path: &Path) -> Option<AuthoritativeCodexEvidence> {
    CodexAdapter
        .inspect_authoritative_evidence_with_hook(path, &fixture.cwd, &fixture.data_home, || {})
        .unwrap()
}

#[test]
fn retained_object_proof_allows_normal_transcript_appends() {
    let fixture = Fixture::new();
    let path = fixture.write_main(SESSION_ID);
    let proof = capture(&fixture, &path).unwrap();
    assert!(proof.verify().unwrap());
    OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(APPEND_EVENT)
        .unwrap();
    assert!(proof.verify().unwrap());
    assert!(proof.same_evidence(&capture(&fixture, &path).unwrap()));
}

#[test]
fn same_value_file_replacement_does_not_reuse_the_retained_proof() {
    let fixture = Fixture::new();
    let path = fixture.write_main(SESSION_ID);
    let proof = capture(&fixture, &path).unwrap();
    fs::rename(&path, fixture.rollout_directory.join(DISPLACED_FILE)).unwrap();
    fixture.write_main(SESSION_ID);
    let replacement = capture(&fixture, &path).unwrap();
    assert_eq!(proof.evidence(), replacement.evidence());
    assert!(!proof.same_evidence(&replacement));
    assert!(!proof.verify().unwrap());
}

#[test]
fn replaced_parent_directory_is_not_equivalent_provider_evidence() {
    let fixture = Fixture::new();
    let path = fixture.write_main(SESSION_ID);
    let proof = capture(&fixture, &path).unwrap();
    fs::rename(
        &fixture.rollout_directory,
        fixture.data_home.join(DISPLACED_DIRECTORY),
    )
    .unwrap();
    fs::create_dir(&fixture.rollout_directory).unwrap();
    fixture.write_main(SESSION_ID);
    assert!(!proof.verify().unwrap());
}

#[test]
fn same_inode_with_changed_session_metadata_is_rejected() {
    let fixture = Fixture::new();
    let path = fixture.write_main(SESSION_ID);
    let proof = capture(&fixture, &path).unwrap();
    fixture.write_values(
        &path,
        &[fixture.metadata(OTHER_SESSION_ID, serde_json::json!(constants::CLI_SOURCE))],
    );
    assert!(!proof.verify().unwrap());
}

#[test]
fn replacement_during_first_read_is_rejected() {
    let fixture = Fixture::new();
    let path = fixture.write_main(SESSION_ID);
    let result = CodexAdapter
        .inspect_authoritative_evidence_with_hook(&path, &fixture.cwd, &fixture.data_home, || {
            fs::rename(&path, fixture.rollout_directory.join(DISPLACED_FILE)).unwrap();
            fixture.write_main(SESSION_ID);
        })
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn missing_and_invalid_rollouts_supply_no_authority() {
    let fixture = Fixture::new();
    assert!(capture(&fixture, &fixture.rollout_directory.join(MISSING_FILE)).is_none());
    assert!(capture(&fixture, &fixture.evidence(SESSION_ID)).is_none());
    let path = fixture.write_main(SESSION_ID);
    let proof = capture(&fixture, &path).unwrap();
    fs::rename(&path, fixture.rollout_directory.join(DISPLACED_FILE)).unwrap();
    assert!(!proof.verify().unwrap());
}
