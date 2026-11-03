use super::super::validation::test_fixture::Fixture;
use super::*;
use crate::sha256_digest::Sha256Digest;
use uuid::Uuid;

const SCHEMA_STEP: u16 = 1;
const GENERATION_STEP: u64 = 1;
const FIRST_TIMESTAMP: i64 = 1;
const RELATIVE_PATH: &str = "relative";
const SIZE_STEP: u16 = 1;

#[test]
fn migration_runtime_rejects_every_independent_identity_and_shape_change() {
    let fixture = Fixture::new();
    let intent = fixture.intent;
    let original = &intent.runtime.target_payload;
    let mutations: &[fn(&mut RuntimeRecord)] = &[
        |record| record.schema_version += SCHEMA_STEP,
        |record| record.terminal_id = Uuid::new_v4(),
        |record| record.runtime_epoch = Uuid::new_v4(),
        |record| record.worker_id = Uuid::new_v4(),
        |record| record.controller_generation += GENERATION_STEP,
        |record| record.worker_pid = u32::default(),
        |record| record.worker_start_time = u64::default(),
        |record| record.child_pid = u32::default(),
        |record| record.child_start_time = u64::default(),
        |record| record.child_pid = record.worker_pid,
        |record| record.started_at_ms = i64::default(),
        |record| record.socket_path = RELATIVE_PATH.into(),
        |record| record.auth_token.clear(),
        |record| record.command.clear(),
        |record| record.cwd = RELATIVE_PATH.into(),
        |record| record.cols = crate::model::MIN_TERMINAL_COLUMNS - SIZE_STEP,
        |record| record.rows = crate::model::MIN_TERMINAL_ROWS - SIZE_STEP,
        |record| record.boot_id.clear(),
    ];
    assert!(runtime_matches_intent(
        original,
        &intent,
        &intent.target_boot_id
    ));
    validate_runtime(&intent).unwrap();
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(
            !runtime_matches_intent(&changed, &intent, &intent.target_boot_id),
            "mutation {index}"
        );
        let mut tampered = intent.clone();
        tampered.runtime.target_sha256 = Sha256Digest::capture(&changed).unwrap();
        tampered.runtime.target_payload = changed;
        assert!(validate_runtime(&tampered).is_err(), "mutation {index}");
    }
}

#[test]
fn minimal_runtime_dimensions_and_different_source_boot_reconstruct_exactly() {
    let fixture = Fixture::new();
    let mut intent = fixture.intent;
    intent.runtime.target_payload.cols = crate::model::MIN_TERMINAL_COLUMNS;
    intent.runtime.target_payload.rows = crate::model::MIN_TERMINAL_ROWS;
    intent.runtime.target_payload.started_at_ms = FIRST_TIMESTAMP;
    let mut source = intent.runtime.target_payload.clone();
    source.boot_id.clone_from(&intent.source_boot_id);
    intent.runtime.source_sha256 = Sha256Digest::capture(&source).unwrap();
    intent.runtime.target_sha256 = Sha256Digest::capture(&intent.runtime.target_payload).unwrap();
    assert!(runtime_matches_intent(
        &source,
        &intent,
        &intent.source_boot_id
    ));
    validate_runtime(&intent).unwrap();
    assert!(!runtime_matches_intent(
        &source,
        &intent,
        &intent.target_boot_id
    ));
}
