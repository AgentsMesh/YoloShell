use crate::{constants::*, fixture::*};
use yolo_hub_client_access_protocol::*;

#[test]
fn empty_configuration_and_full_capacity_are_valid() {
    let mut current = configuration();
    current.clients.clear();
    assert_eq!(current.validate(), Ok(()));
    current.clients = (FIRST_INDEX..MAX_CLIENTS).map(grant).collect();
    assert_eq!(current.validate(), Ok(()));
    current.clients.push(grant(MAX_CLIENTS));
    assert_eq!(current.validate(), Err(AccessError::Capacity));
}

#[test]
fn schema_and_revision_fail_closed() {
    for schema in [INVALID_SCHEMA, FUTURE_SCHEMA] {
        let mut current = configuration();
        current.schema_version = schema;
        assert_eq!(current.validate(), Err(AccessError::Configuration));
    }
    let mut current = configuration();
    current.revision = INVALID_REVISION;
    assert_eq!(current.validate(), Err(AccessError::Configuration));
    current.revision = u64::MAX;
    assert_eq!(current.validate(), Ok(()));
}

#[test]
fn duplicate_client_ids_and_duplicate_keys_are_independently_rejected() {
    let original = configuration();
    let mut repeated_id = grant(SECOND_INDEX);
    repeated_id.identity.client_id = original.clients[FIRST_INDEX].identity.client_id;
    let mut repeated_key = grant(SECOND_INDEX);
    repeated_key.identity.public_key = original.clients[FIRST_INDEX].identity.public_key;
    for duplicate in [
        repeated_id,
        repeated_key,
        original.clients[FIRST_INDEX].clone(),
    ] {
        let mut current = original.clone();
        current.clients.push(duplicate);
        assert_eq!(current.validate(), Err(AccessError::DuplicateClient));
    }
}

#[test]
fn configuration_validates_each_grant_without_treating_old_records_as_new_admissions() {
    let mut current = configuration();
    current.clients[FIRST_INDEX].expires_at_ms = NOW_MS;
    assert_eq!(current.validate(), Ok(()));
    assert_eq!(
        current.clients[FIRST_INDEX].validate_at(NOW_MS),
        Err(AccessError::Expired)
    );
    current.clients[FIRST_INDEX].expires_at_ms = INVALID_EXPIRY_MS;
    assert_eq!(current.validate(), Err(AccessError::Grant));
}
