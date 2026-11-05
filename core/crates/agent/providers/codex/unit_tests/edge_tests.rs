use super::*;
use crate::codex::test_support::{self, Fixture};
use serde_json::json;
use std::fs;

const HEAD_PADDING_BYTE: u8 = b'x';
const JSON_LINE_ENDING_BYTE: u8 = b'\n';

#[test]
fn discovers_rollouts_from_each_session() {
    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    let adapter = CodexAdapter;
    assert_eq!(
        adapter.evidence_files(&fixture.context()).unwrap(),
        [evidence.clone()]
    );
}

#[test]
fn filename_metadata_disagreement_and_incomplete_metadata_fail_closed() {
    let fixture = Fixture::new();
    let evidence = fixture.evidence(test_support::SESSION_ID);
    fixture.write_values(
        &evidence,
        &[fixture.metadata(test_support::OTHER_SESSION_ID, json!(constants::CLI_SOURCE))],
    );
    assert!(fixture.inspect(&evidence).is_none());

    let metadata = fixture
        .metadata(test_support::SESSION_ID, json!(constants::CLI_SOURCE))
        .to_string();
    fs::write(&evidence, metadata).unwrap();
    assert!(fixture.inspect(&evidence).is_none());
}

#[test]
fn metadata_beyond_the_bounded_head_is_not_an_identity_anchor() {
    let fixture = Fixture::new();
    let evidence = fixture.evidence(test_support::SESSION_ID);
    let mut contents = vec![
        HEAD_PADDING_BYTE;
        crate::agent::constants::EVIDENCE_HEAD_BYTES
            .try_into()
            .unwrap()
    ];
    contents.push(JSON_LINE_ENDING_BYTE);
    contents.extend_from_slice(
        fixture
            .metadata(test_support::SESSION_ID, json!(constants::CLI_SOURCE))
            .to_string()
            .as_bytes(),
    );
    contents.push(JSON_LINE_ENDING_BYTE);
    fs::write(&evidence, contents).unwrap();
    assert!(fixture.inspect(&evidence).is_none());
}

#[test]
fn only_context_owned_canonical_rollout_paths_are_candidates() {
    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    assert!(CodexAdapter.owns_evidence_path(&evidence, &fixture.context()));
    assert!(CodexAdapter.is_evidence_candidate_path(&evidence));

    let uppercase = fixture.evidence(&test_support::SESSION_ID.to_uppercase());
    fs::write(&uppercase, fs::read(&evidence).unwrap()).unwrap();
    assert!(!CodexAdapter.is_evidence_candidate_path(&uppercase));
    assert!(fixture.inspect(&uppercase).is_none());
}
