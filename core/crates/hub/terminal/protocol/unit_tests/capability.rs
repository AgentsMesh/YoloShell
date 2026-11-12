use crate::fixtures::{constants::*, roundtrip};
use uuid::Uuid;
use yolo_hub_terminal_protocol::*;

#[test]
fn capabilities_are_random_canonical_and_redacted() {
    let capability = SecretCapability::generate();
    assert_ne!(capability, SecretCapability::generate());
    assert_eq!(
        SecretCapability::from_uuid(capability.as_uuid()).unwrap(),
        capability
    );
    roundtrip(&capability);
    let debug = format!("{capability:?}");
    assert!(debug.contains(REDACTED));
    assert!(!debug.contains(&capability.as_uuid().to_string()));
    assert_eq!(String::from(capability), capability.as_uuid().to_string());
}

#[test]
fn reject_malformed_nonrandom_and_noncanonical_capabilities() {
    for value in [
        NIL_ID,
        INVALID_ID,
        CAPABILITY_UPPER,
        CAPABILITY_COMPACT,
        NON_RANDOM_ID,
        NON_RFC_ID,
    ] {
        assert_eq!(
            SecretCapability::try_from(value.to_owned()),
            Err(PublicError::InvalidCapability)
        );
        assert!(serde_json::from_value::<SecretCapability>(value.into()).is_err());
    }
    assert_eq!(
        SecretCapability::from_uuid(Uuid::nil()),
        Err(PublicError::InvalidCapability)
    );
}
