use super::retirement_test_constants::*;
use super::retirement_test_support::{RetirementFixture, replace_same_content};
use crate::codex_hooks::CodexRetirementRecord;
use serde_json::Value;

#[derive(Clone, Copy)]
enum RecordMutation {
    AdmissionFileIdentity,
    Boot,
    RecordSchemaVersion,
    SealDigest,
    SealFileIdentity,
    SealSchemaVersion,
    SnapshotDigest,
    SnapshotLineage,
}

const RECORD_MUTATIONS: &[RecordMutation] = &[
    RecordMutation::AdmissionFileIdentity,
    RecordMutation::Boot,
    RecordMutation::RecordSchemaVersion,
    RecordMutation::SealDigest,
    RecordMutation::SealFileIdentity,
    RecordMutation::SealSchemaVersion,
    RecordMutation::SnapshotDigest,
    RecordMutation::SnapshotLineage,
];

#[test]
fn same_content_seal_replacement_revokes_the_old_record_lineage() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = fixture.seal();
    let original = writer.publish_seal(&seal, fixture.admission()).unwrap();
    let terminal = fixture.terminal(&writer, original.artifact());

    replace_same_content(&fixture.seal_path());
    let replacement = writer.publish_seal(&seal, fixture.admission()).unwrap();

    assert!(replacement.is_replay());
    assert_ne!(
        original.artifact().file_identity(),
        replacement.artifact().file_identity()
    );
    assert!(
        writer
            .verify_seal(original.artifact(), fixture.admission())
            .is_err()
    );
    assert!(
        terminal
            .record
            .validate_for(
                &terminal.snapshot,
                fixture.admission(),
                replacement.artifact(),
                &terminal.proof,
            )
            .is_err()
    );
}

#[test]
fn every_persisted_lineage_mutation_fails_closed() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = writer
        .publish_seal(&fixture.seal(), fixture.admission())
        .unwrap();
    let terminal = fixture.terminal(&writer, seal.artifact());
    let encoded = serde_json::to_value(&terminal.record).unwrap();

    for mutation in RECORD_MUTATIONS {
        let changed = mutate_record(encoded.clone(), *mutation);
        let decoded: CodexRetirementRecord = serde_json::from_value(changed).unwrap();
        assert!(
            decoded
                .validate_for(
                    &terminal.snapshot,
                    fixture.admission(),
                    seal.artifact(),
                    &terminal.proof,
                )
                .is_err()
        );
    }
}

#[test]
fn unknown_fields_are_rejected_at_every_lineage_layer() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = writer
        .publish_seal(&fixture.seal(), fixture.admission())
        .unwrap();
    let terminal = fixture.terminal(&writer, seal.artifact());
    let encoded = serde_json::to_value(terminal.record).unwrap();

    for path in UNKNOWN_FIELD_LOCATIONS {
        let mut changed = encoded.clone();
        object_at(&mut changed, path).insert(UNKNOWN_FIELD.to_owned(), Value::Bool(UNKNOWN_VALUE));
        assert!(serde_json::from_value::<CodexRetirementRecord>(changed).is_err());
    }
}

fn mutate_record(mut value: Value, mutation: RecordMutation) -> Value {
    match mutation {
        RecordMutation::AdmissionFileIdentity => {
            increment_unsigned(&mut value[SEAL_FIELD][ADMISSION_FILE_IDENTITY_FIELD][INODE_FIELD])
        }
        RecordMutation::Boot => {
            value[SEAL_FIELD][BOOT_FIELD][KNOWN_FIELD] = Value::Bool(false);
        }
        RecordMutation::RecordSchemaVersion => {
            increment_unsigned(&mut value[SCHEMA_VERSION_FIELD]);
        }
        RecordMutation::SealDigest => mutate_digest(&mut value[SEAL_SHA256_FIELD]),
        RecordMutation::SealFileIdentity => {
            increment_unsigned(&mut value[SEAL_FILE_IDENTITY_FIELD][INODE_FIELD]);
        }
        RecordMutation::SealSchemaVersion => {
            increment_unsigned(&mut value[SEAL_FIELD][SCHEMA_VERSION_FIELD]);
        }
        RecordMutation::SnapshotDigest => mutate_digest(&mut value[SNAPSHOT_SHA256_FIELD]),
        RecordMutation::SnapshotLineage => {
            increment_unsigned(&mut value[SNAPSHOT_LINEAGE_FIELD][SNAPSHOT_FIELD][INODE_FIELD])
        }
    }
    value
}

fn increment_unsigned(value: &mut Value) {
    let original = value.as_u64().unwrap();
    *value = Value::from(original.wrapping_add(UNSIGNED_OFFSET));
}

fn mutate_digest(value: &mut Value) {
    let byte = value[DIGEST_BYTE_INDEX].as_u64().unwrap() as u8;
    value[DIGEST_BYTE_INDEX] = Value::from(byte.wrapping_add(DIGEST_BYTE_OFFSET));
}

fn object_at<'a>(
    mut value: &'a mut Value,
    path: &[&str],
) -> &'a mut serde_json::Map<String, Value> {
    for field in path {
        value = value.get_mut(field).unwrap();
    }
    value.as_object_mut().unwrap()
}
