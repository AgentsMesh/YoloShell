use super::*;
use yolo_wire::field;

const LEASE_FIELD: &str = "lease";
const TEST_HOLDER: &str = "desktop";
const TEST_CONTROLLER: &str = "main";
const TEST_GENERATION: u64 = 7;
const TEST_EXPIRY_MS: i64 = 8;
const TEST_TTL_MS: u64 = 9;
const TEST_RENEW_AFTER_MS: u64 = 3;
const NIL_UUID: &str = "00000000-0000-0000-0000-000000000000";
const NON_V4_UUID: &str = "f47ac10b-58cc-11cf-a447-001122334455";
const NON_RFC_V4_UUID: &str = "f47ac10b-58cc-4fcf-2447-001122334455";

#[test]
fn capability_wire_is_a_redacted_uuid_string() {
    let capability = WriterCapability::generate();
    let wire = serde_json::to_string(&capability).unwrap();
    assert!(serde_json::from_str::<String>(&wire).is_ok());
    assert_eq!(
        serde_json::from_str::<WriterCapability>(&wire).unwrap(),
        capability
    );
    let debug = format!("{capability:?}");
    assert!(debug.contains(REDACTED_CAPABILITY));
    assert!(!debug.contains(wire.trim_matches('"')));
}

#[test]
fn invalid_nil_and_non_v4_capabilities_are_rejected() {
    for wire in ["not-a-uuid", NIL_UUID, NON_V4_UUID, NON_RFC_V4_UUID] {
        assert!(serde_json::from_value::<WriterCapability>(wire.into()).is_err());
    }
    assert!(serde_json::from_value::<WriterCapability>(42.into()).is_err());
    assert!(serde_json::from_value::<WriterCapability>(serde_json::Value::Null).is_err());
}

#[test]
fn grant_flattens_the_public_lease() {
    let grant = WriterLeaseGrant {
        lease: WriterLease {
            holder_id: TEST_HOLDER.into(),
            controller_instance_id: Some(TEST_CONTROLLER.into()),
            generation: TEST_GENERATION,
            expires_at_ms: TEST_EXPIRY_MS,
            ttl_ms: TEST_TTL_MS,
            renew_after_ms: TEST_RENEW_AFTER_MS,
        },
        writer_capability: WriterCapability::generate(),
    };
    let value = serde_json::to_value(&grant).unwrap();
    assert_eq!(value[field::HOLDER_ID], TEST_HOLDER);
    assert!(value.get(LEASE_FIELD).is_none());
    assert!(value[field::WRITER_CAPABILITY].is_string());
    assert_eq!(
        serde_json::from_value::<WriterLeaseGrant>(value.clone()).unwrap(),
        grant
    );
    let secret = value[field::WRITER_CAPABILITY].as_str().unwrap();
    let debug = format!("{grant:?}");
    assert!(debug.contains(REDACTED_CAPABILITY));
    assert!(!debug.contains(secret));
}
