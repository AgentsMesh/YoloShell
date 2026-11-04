use super::fixtures::{self, constants::*, *};
use crate::ConfigurationChange;

#[test]
fn changes_keep_correlation_separate_from_expected_revision() {
    let change = fixtures::change();
    change.validate(identity()).unwrap();
    roundtrip(&change);
    let mut exhausted = change.clone();
    exhausted.expected_revision = u64::MAX;
    exhausted.validate(identity()).unwrap();
    roundtrip(&exhausted);
}

#[test]
fn nil_request_and_zero_revision_are_invalid_in_memory_and_during_deserialization() {
    let mut change = fixtures::change();
    change.request_id = uuid::Uuid::nil();
    assert!(change.validate(identity()).is_err());
    let mut change = fixtures::change();
    change.expected_revision = 0;
    assert!(change.validate(identity()).is_err());
    reject(&fixtures::change(), EXPECTED_REVISION_FIELD, 0.into());
    for invalid in [NIL_ID, MALFORMED_ID, UPPER_ID, COMPACT_ID] {
        reject(&fixtures::change(), REQUEST_FIELD, invalid.into());
    }
}

#[test]
fn changes_reject_self_peering_and_unknown_fields_without_mutating_the_request() {
    let mut change = fixtures::change();
    change.configuration.peers[0].identity = identity();
    assert!(change.validate(identity()).is_err());
    let change = fixtures::change();
    reject(&change, UNKNOWN_FIELD, UNKNOWN_VALUE.into());
    let mut wire = serde_json::to_value(&change).unwrap();
    wire[CONFIGURATION_FIELD][UNKNOWN_FIELD] = UNKNOWN_VALUE.into();
    assert!(serde_json::from_value::<ConfigurationChange>(wire).is_err());
}
