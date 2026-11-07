use super::super::constants::test_field;
use super::super::{OptionalParticipant, RebootMigrationIntent};
use super::{test_fixture::Fixture, validate};
use crate::sha256_digest::Sha256Digest;
use serde::Serialize;

const AUTHORITIES_FIELD: &str = "authorities";
const FIRST_AUTHORITY_INDEX: usize = 0;
const FIRST_RECORD_INDEX: usize = 0;
const FOREIGN_WORKER_PID: u32 = u32::MAX;
const PID_INCREMENT: u32 = 1;
const RECORDS_FIELD: &str = "records";
const RUNTIME_FIELD: &str = "runtime";
const REVISION_INCREMENT: u64 = 1;
const REVISION_FIELD: &str = "revision";
const STATE_FIELD: &str = crate::protocol::SERDE_STATE_TAG;
const UNKNOWN_FIELD: &str = "unexpected";
const WORKER_FIELD: &str = "worker";
const WORKER_PID_FIELD: &str = "pid";
const PROCESS_IDENTITY_INCREMENT: u32 = 1;
const PROCESS_START_INCREMENT: u64 = 1;

#[derive(Clone, Copy)]
enum PathSegment<'a> {
    Field(&'a str),
    Index(usize),
}

#[test]
fn nested_unknown_fields_fail_at_typed_schema_boundary() {
    for (participant, suffix) in [
        (test_field::STARTUP_CHECKPOINT, &[][..]),
        (
            test_field::CLAUDE_CUSTODY,
            &[
                PathSegment::Field(RECORDS_FIELD),
                PathSegment::Index(FIRST_RECORD_INDEX),
            ][..],
        ),
        (
            test_field::CLAUDE_GENERATION,
            &[
                PathSegment::Field(AUTHORITIES_FIELD),
                PathSegment::Index(FIRST_AUTHORITY_INDEX),
            ][..],
        ),
        (
            test_field::PTY_CUSTODY,
            &[PathSegment::Field(STATE_FIELD)][..],
        ),
    ] {
        let fixture = Fixture::new();
        let mut value = serde_json::to_value(&fixture.intent).unwrap();
        let mut path = vec![
            PathSegment::Field(participant),
            PathSegment::Field(test_field::TARGET_PAYLOAD),
        ];
        path.extend_from_slice(suffix);
        object_at_mut(&mut value, &path)
            .insert(UNKNOWN_FIELD.to_owned(), serde_json::Value::Bool(true));
        assert!(
            serde_json::from_value::<RebootMigrationIntent>(value).is_err(),
            "participant {participant} accepted an unknown nested field"
        );
    }
}

#[test]
fn recomputed_digests_cannot_authorize_non_boot_participant_mutations() {
    assert_recomputed_digest_rejected(test_field::STARTUP_CHECKPOINT, |payload| {
        payload[WORKER_FIELD][WORKER_PID_FIELD] = serde_json::json!(FOREIGN_WORKER_PID);
    });
    assert_recomputed_digest_rejected(test_field::CLAUDE_CUSTODY, |payload| {
        payload[RECORDS_FIELD][FIRST_RECORD_INDEX][WORKER_FIELD][WORKER_PID_FIELD] =
            serde_json::json!(FOREIGN_WORKER_PID);
    });
    assert_recomputed_digest_rejected(test_field::CLAUDE_GENERATION, |payload| {
        let revision = payload[AUTHORITIES_FIELD][FIRST_AUTHORITY_INDEX][REVISION_FIELD]
            .as_u64()
            .unwrap();
        payload[AUTHORITIES_FIELD][FIRST_AUTHORITY_INDEX][REVISION_FIELD] =
            serde_json::json!(revision + REVISION_INCREMENT);
    });
    assert_recomputed_digest_rejected(test_field::PTY_CUSTODY, |payload| {
        let pid = payload[RUNTIME_FIELD][test_field::WORKER_PID]
            .as_u64()
            .unwrap();
        payload[RUNTIME_FIELD][test_field::WORKER_PID] =
            serde_json::json!(pid + u64::from(PID_INCREMENT));
    });
}

#[test]
fn self_consistent_checkpoint_digests_cannot_cross_runtime_process_identity() {
    assert_checkpoint_process_mismatch_rejected(|checkpoint| {
        checkpoint.worker.pid += PROCESS_IDENTITY_INCREMENT;
    });
    assert_checkpoint_process_mismatch_rejected(|checkpoint| {
        checkpoint.child.as_mut().unwrap().start_time += PROCESS_START_INCREMENT;
    });
}

fn assert_checkpoint_process_mismatch_rejected(
    mutate: impl FnOnce(&mut crate::worker::StartupCheckpointEvidence),
) {
    let mut intent = Fixture::new().intent;
    let OptionalParticipant::Replace(participant) = &mut intent.startup_checkpoint else {
        unreachable!();
    };
    mutate(&mut participant.target_payload);
    participant.target_sha256 = Sha256Digest::capture(&participant.target_payload).unwrap();
    let mut source = participant.target_payload.clone();
    source.boot_id.clone_from(&intent.source_boot_id);
    participant.source_sha256 = Sha256Digest::capture(&source).unwrap();
    assert!(validate(&intent).is_err());
}

fn assert_recomputed_digest_rejected(
    participant: &str,
    mutate: impl FnOnce(&mut serde_json::Value),
) {
    let fixture = Fixture::new();
    let mut value = serde_json::to_value(&fixture.intent).unwrap();
    mutate(&mut value[participant][test_field::TARGET_PAYLOAD]);
    let digest = digest_participant(participant, &value[participant][test_field::TARGET_PAYLOAD]);
    value[participant][test_field::TARGET_SHA256] = serde_json::to_value(digest).unwrap();
    let tampered: RebootMigrationIntent = serde_json::from_value(value).unwrap();
    assert!(validate(&tampered).is_err());
}

fn digest_participant(participant: &str, payload: &serde_json::Value) -> Sha256Digest {
    match participant {
        test_field::STARTUP_CHECKPOINT => {
            typed_digest::<crate::worker::StartupCheckpointEvidence>(payload)
        }
        test_field::CLAUDE_CUSTODY => typed_digest::<crate::claude_custody::CustodyLedger>(payload),
        test_field::CLAUDE_GENERATION => {
            typed_digest::<crate::agent::claude::generation::ClaudeInvocationAuthorities>(payload)
        }
        test_field::PTY_CUSTODY => {
            typed_digest::<crate::pty_custody::CustodyDurableRecord>(payload)
        }
        _ => unreachable!(),
    }
}

fn typed_digest<T>(payload: &serde_json::Value) -> Sha256Digest
where
    T: serde::de::DeserializeOwned + Serialize,
{
    Sha256Digest::capture(&serde_json::from_value::<T>(payload.clone()).unwrap()).unwrap()
}

fn object_at_mut<'a>(
    mut value: &'a mut serde_json::Value,
    path: &[PathSegment<'_>],
) -> &'a mut serde_json::Map<String, serde_json::Value> {
    for segment in path {
        value = match segment {
            PathSegment::Field(field) => &mut value[*field],
            PathSegment::Index(index) => &mut value[*index],
        };
    }
    value.as_object_mut().unwrap()
}

#[test]
fn fixture_contains_every_typed_participant() {
    let intent = Fixture::new().intent;
    assert!(matches!(
        intent.startup_checkpoint,
        OptionalParticipant::Replace(_)
    ));
    assert!(matches!(
        intent.claude_custody,
        OptionalParticipant::Replace(_)
    ));
    assert!(matches!(
        intent.claude_generation,
        OptionalParticipant::Replace(_)
    ));
    assert!(matches!(
        intent.pty_custody,
        OptionalParticipant::Replace(_)
    ));
    assert!(validate(&intent).is_ok());
}
