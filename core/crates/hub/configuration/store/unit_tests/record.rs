use super::{constants::*, support::*};
use crate::{RevisionConflict, record::Record};
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};

fn record() -> Record {
    Record {
        schema_version: crate::constants::PRIVATE_RECORD_SCHEMA,
        directory: directory_identity(),
        state: state(identity()),
        _legacy_listener_binding: false,
    }
}

#[test]
fn state_is_bound_to_exact_directory_hub_and_public_key() {
    let record = record();
    record
        .validate(record.directory, record.state.identity)
        .unwrap();
    let other_directory = directory_identity();
    assert!(
        record
            .validate(other_directory, record.state.identity)
            .is_err()
    );
    assert!(record.validate(record.directory, identity()).is_err());
    let other_key = PeerIdentity::new(
        record.state.identity.hub(),
        [OTHER_KEY_BYTE; ED25519_PUBLIC_KEY_BYTES],
    )
    .unwrap();
    assert!(record.validate(record.directory, other_key).is_err());
    let restored: Record = serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();
    assert_eq!(restored, record);
}

#[test]
fn malformed_state_and_unknown_record_fields_do_not_gain_authority() {
    let record = record();
    let mutations: &[fn(&mut Record)] = &[
        |r| r.schema_version = u16::MAX,
        |r| r.state.schema_version = u16::MAX,
        |r| r.state.revision = u64::MIN,
        |r| r.state.last_change = uuid::Uuid::nil(),
        |r| r.state.configuration.trusted_origins.push(r.state.identity),
    ];
    for mutate in mutations {
        let mut candidate = record.clone();
        mutate(&mut candidate);
        assert!(
            candidate
                .validate(record.directory, record.state.identity)
                .is_err()
        );
    }
    let original = serde_json::to_value(&record).unwrap();
    for path in [
        Vec::new(),
        vec![STATE_FIELD],
        vec![STATE_FIELD, CONFIGURATION_FIELD],
    ] {
        let mut wire = original.clone();
        let mut value = &mut wire;
        for field in path {
            value = value.get_mut(field).unwrap();
        }
        value
            .as_object_mut()
            .unwrap()
            .insert(INJECTED_FIELD.into(), true.into());
        assert!(serde_json::from_value::<Record>(wire).is_err());
    }
    assert!(original.get(DIRECTORY_FIELD).is_some());
    assert!(original[STATE_FIELD].get(SCHEMA_FIELD).is_some());
    assert!(original[STATE_FIELD].get(IDENTITY_FIELD).is_some());
    assert!(original[STATE_FIELD].get(REVISION_FIELD).is_some());
    assert!(original[STATE_FIELD].get(LAST_CHANGE_FIELD).is_some());
}

#[test]
fn revision_conflict_is_a_typed_error_and_change_fixtures_are_valid() {
    let identity = identity();
    let state = state(identity);
    let change = change(&state);
    change.validate(identity).unwrap();
    assert_eq!(change.expected_revision, state.revision);
    assert_ne!(change.request_id, state.last_change);
    assert_ne!(change.configuration, state.configuration);
    let error = anyhow::Error::new(RevisionConflict);
    assert!(error.downcast_ref::<RevisionConflict>().is_some());
    assert!(!error.to_string().is_empty());
}
