use super::*;

const KNOWN_BOOT: &str = "test-known-boot";
const UNKNOWN_BOOT: &str = "unknown:test-boot";
const FORMAT_FIELD: &str = "format";
const UNSUPPORTED_FORMAT: &str = "boot-id-sha256-v0";
const UNSUPPORTED_PREFIX: &str = "boot-v0-";
const INVALID_HEX: char = 'g';

#[test]
fn encoded_identity_requires_current_format_and_exact_lowercase_digest() {
    let identity = BootIdentity::from_value(KNOWN_BOOT);
    let encoded = identity.encoded();
    assert_eq!(
        BootIdentity::from_encoded(identity.known(), &encoded),
        Some(identity)
    );
    let raw = encoded.strip_prefix(ENCODED_PREFIX).unwrap();
    for malformed in [
        raw.to_owned(),
        format!("{UNSUPPORTED_PREFIX}{raw}"),
        format!("{ENCODED_PREFIX}{}", raw.to_uppercase()),
        format!(
            "{ENCODED_PREFIX}{}",
            INVALID_HEX.to_string().repeat(DIGEST_HEX_LENGTH)
        ),
        format!("{encoded}{INVALID_HEX}"),
        ENCODED_PREFIX.to_owned(),
    ] {
        for known in [false, true] {
            assert!(
                BootIdentity::from_encoded(known, &malformed).is_none(),
                "{malformed}"
            );
        }
    }
}

#[test]
fn serialized_identity_rejects_missing_unknown_and_null_provenance() {
    let original = BootIdentity::from_value(KNOWN_BOOT);
    let encoded = serde_json::to_value(original).unwrap();
    assert_eq!(
        serde_json::from_value::<BootIdentity>(encoded.clone()).unwrap(),
        original
    );
    let mut untyped = encoded.clone();
    untyped.as_object_mut().unwrap().remove(FORMAT_FIELD);
    assert!(serde_json::from_value::<BootIdentity>(untyped).is_err());
    for format in [serde_json::Value::Null, UNSUPPORTED_FORMAT.into()] {
        let mut changed = encoded.clone();
        changed[FORMAT_FIELD] = format;
        assert!(serde_json::from_value::<BootIdentity>(changed).is_err());
    }
}

#[test]
fn current_digest_cannot_equal_the_untagged_hash_of_the_same_boot() {
    let current = BootIdentity::from_value(KNOWN_BOOT);
    let untyped: [u8; DIGEST_BYTES] = Sha256::digest(KNOWN_BOOT.as_bytes()).into();
    assert_ne!(current.digest, untyped);
}

#[test]
fn current_boot_and_unknown_boot_keep_their_relationship_after_round_trip() {
    for original in [
        BootIdentity::current(),
        BootIdentity::from_value(UNKNOWN_BOOT),
    ] {
        let decoded = BootIdentity::from_encoded(original.known(), &original.encoded()).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(
            decoded.relation_to_current(),
            original.relation_to_current()
        );
    }
    assert_eq!(
        BootIdentity::from_value(UNKNOWN_BOOT).relation_to_current(),
        BootIdentityRelation::Unknown
    );
}
