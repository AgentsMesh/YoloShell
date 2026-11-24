use crate::{constants::*, fixture::*};
use yolo_hub_client_access_protocol::*;

#[test]
fn identities_require_both_a_non_nil_client_id_and_nonzero_key() {
    let valid = identity(FIRST_INDEX);
    assert_eq!(valid.validate(), Ok(()));
    let mut invalid = valid;
    invalid.client_id = uuid::Uuid::nil();
    assert_eq!(invalid.validate(), Err(AccessError::Identity));
    invalid = valid;
    invalid.public_key = [u8::MIN; ED25519_PUBLIC_KEY_BYTES];
    assert_eq!(invalid.validate(), Err(AccessError::Identity));
    invalid.public_key[ED25519_PUBLIC_KEY_BYTES - ONE] = KEY_FILL;
    assert_eq!(invalid.validate(), Ok(()));
}

#[test]
fn errors_have_distinct_public_diagnostics_without_nested_secrets() {
    let errors = [
        AccessError::Identity,
        AccessError::Grant,
        AccessError::Configuration,
        AccessError::DuplicateClient,
        AccessError::DuplicateOrigin,
        AccessError::Capacity,
        AccessError::Expired,
    ];
    let mut messages = std::collections::BTreeSet::new();
    for error in errors {
        assert!(!error.to_string().is_empty());
        assert!(messages.insert(error.to_string()));
        assert!(std::error::Error::source(&error).is_none());
    }
}
