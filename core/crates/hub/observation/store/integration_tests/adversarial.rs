use crate::{constants::*, fixture::*, integration_constants::*, integration_helpers::*};
use serde_json::Value;
use std::{fs, num::NonZeroUsize};
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_observation_store::ObservationStore;
use yolo_hub_peer_protocol_test_support::support::Identity;
use yolo_paths::{constants::MAX_DURABLE_JSON_BYTES, directory::PrivateDirectory};

#[test]
fn missing_indexed_origin_fails_closed_and_does_not_recreate_the_record() {
    let fixture = committed();
    let path = fixture.record_path(fixture.peer.origin.identity.hub());
    let index = file_state(&fixture.index_path());
    fs::rename(&path, fixture.sibling(DISPLACED)).unwrap();
    assert!(fixture.open().is_err());
    assert!(!path.exists());
    assert_eq!(file_state(&fixture.index_path()), index);
}

#[test]
fn malformed_indexed_records_are_rejected_without_repair_or_deletion() {
    let fixture = committed();
    let path = fixture.record_path(fixture.peer.origin.identity.hub());
    let valid = document(&path);
    let mut bad_signature = valid.clone();
    bad_signature[CATALOGUE_KEY][SIGNATURE_KEY] = Value::Array(Vec::new());
    let mut tampered_payload = valid.clone();
    tampered_payload[CATALOGUE_KEY][PAYLOAD_KEY][SESSIONS_KEY] = Value::Array(Vec::new());
    let mut grants = valid.clone();
    grants[GRANTS_KEY] = Value::Array(Vec::new());
    let mut unsupported = valid.clone();
    unsupported[SCHEMA_KEY] = Value::from(UNSUPPORTED_SCHEMA);
    for value in [bad_signature, tampered_payload, grants, unsupported] {
        reject_unchanged(&fixture, &path, &serde_json::to_vec(&value).unwrap());
    }
    reject_unchanged(&fixture, &path, INVALID_BYTES);
    reject_unchanged(&fixture, &path, DUPLICATE_SCHEMA);
    let oversized = vec![JSON_SPACE; usize::try_from(MAX_DURABLE_JSON_BYTES + STEP).unwrap()];
    reject_unchanged(&fixture, &path, &oversized);
}

#[test]
fn a_complete_record_with_a_duplicate_json_key_is_rejected() {
    let fixture = committed();
    let path = fixture.record_path(fixture.peer.origin.identity.hub());
    let valid = String::from_utf8(fs::read(&path).unwrap()).unwrap();
    let value = document(&path)[SCHEMA_KEY].to_string();
    let duplicate = format!("{INDEX_SCHEMA_DUPLICATE}:{value},{INDEX_SCHEMA_DUPLICATE}");
    let duplicated = valid.replacen(INDEX_SCHEMA_DUPLICATE, &duplicate, ONE_ORIGIN);
    reject_unchanged(&fixture, &path, duplicated.as_bytes());
}

#[test]
fn invalid_index_schema_unknown_authority_duplicate_origins_and_bad_json_fail_closed() {
    let fixture = committed();
    let path = fixture.index_path();
    let valid = document(&path);
    let mut duplicate = valid.clone();
    let origins = duplicate[INDEX_ORIGINS].as_array_mut().unwrap();
    origins.extend(origins.clone());
    let mut unsupported = valid.clone();
    unsupported[SCHEMA_KEY] = Value::from(UNSUPPORTED_SCHEMA);
    let mut grants = valid;
    grants[GRANTS_KEY] = Value::Array(Vec::new());
    for value in [duplicate, unsupported, grants] {
        reject_unchanged(&fixture, &path, &serde_json::to_vec(&value).unwrap());
    }
    reject_unchanged(&fixture, &path, INVALID_BYTES);
    reject_unchanged(&fixture, &path, DUPLICATE_SCHEMA);
}

#[test]
fn smaller_open_capacity_and_unsorted_index_cannot_import_a_larger_history() {
    let fixture = committed();
    let other = Identity::new();
    fixture
        .open()
        .unwrap()
        .record_history(&fixture.with(vec![empty_catalogue(&other)]))
        .unwrap();
    let mut limits = routing_limits();
    limits.max_known_agents = NonZeroUsize::new(ONE_ORIGIN).unwrap();
    let path = fixture.index_path();
    let before = file_state(&path);
    assert!(
        ObservationStore::open(
            PrivateDirectory::open(&fixture.path).unwrap(),
            fixture.peer.entry.identity,
            limits
        )
        .is_err()
    );
    assert_eq!(file_state(&path), before);
    let mut unsorted = document(&path);
    unsorted[INDEX_ORIGINS].as_array_mut().unwrap().reverse();
    reject_unchanged(&fixture, &path, &serde_json::to_vec(&unsorted).unwrap());
}

#[test]
fn oversized_incoming_origin_is_rejected_before_any_write_and_does_not_poison_storage() {
    let fixture = committed();
    let large_origin = Identity::new();
    let oversized = fixture.with(vec![large_catalogue(&large_origin, OVERSIZED_SESSIONS)]);
    let before = file_state(&fixture.index_path());
    let mut store = fixture.open().unwrap();
    assert!(store.record_history(&oversized).is_err());
    assert_eq!(file_state(&fixture.index_path()), before);
    assert!(!fixture.record_path(large_origin.identity.hub()).exists());
    assert_eq!(store.history().unwrap(), fixture.history());
    let small = fixture.with(vec![empty_catalogue(&large_origin)]);
    store.record_history(&small).unwrap();
    assert_eq!(store.history().unwrap().catalogues().len(), TWO_ORIGINS);
}

#[test]
fn missing_index_in_a_nonempty_directory_does_not_initialize_or_adopt_unknown_files() {
    let fixture = Fixture::new();
    let path = fixture.path.join(UNKNOWN);
    write_new(&path, INVALID_BYTES);
    let before = file_state(&path);
    assert!(fixture.open().is_err());
    assert!(!fixture.index_path().exists());
    assert_eq!(file_state(&path), before);
}
