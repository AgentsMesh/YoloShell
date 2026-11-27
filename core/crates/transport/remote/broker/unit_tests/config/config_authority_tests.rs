use super::super::constants::PROFILE_AUTHORITY_SHA256_CONFIG_FIELD;
use super::super::{BrokerConfig, BrokerPaths, stage_broker_config};
use super::config_tests::ConfigFixture;
use super::support::{HOST_ID, PROFILE_AUTHORITY_SHA256};
use serde_json::json;

const REPLACEMENT_PROFILE_AUTHORITY_SHA256: &str =
    "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const INVALID_PROFILE_AUTHORITY_SHA256: &str = "0";
const UPPERCASE_PROFILE_AUTHORITY_SHA256: &str =
    "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";
const AUTHORITY_STATE_DIRECTORY: &str = "authority-state";
const REVISION_STATE_DIRECTORY: &str = "revision-state";

#[test]
fn profile_authority_digest_is_required_and_validated() {
    let fixture = ConfigFixture::new();
    let loaded = BrokerConfig::load(fixture.write(&fixture.value())).unwrap();
    assert_eq!(loaded.profile_authority_sha256(), PROFILE_AUTHORITY_SHA256);

    let mut missing = fixture.value();
    missing
        .as_object_mut()
        .unwrap()
        .remove(PROFILE_AUTHORITY_SHA256_CONFIG_FIELD);
    assert!(BrokerConfig::load(fixture.write(&missing)).is_err());

    for invalid in [
        INVALID_PROFILE_AUTHORITY_SHA256,
        UPPERCASE_PROFILE_AUTHORITY_SHA256,
    ] {
        let mut value = fixture.value();
        value[PROFILE_AUTHORITY_SHA256_CONFIG_FIELD] = json!(invalid);
        assert!(BrokerConfig::load(fixture.write(&value)).is_err());
    }
}

#[test]
fn immutable_generation_preserves_profile_authority_binding() {
    let fixture = ConfigFixture::new();
    let paths = BrokerPaths::new(fixture.root.path().join(AUTHORITY_STATE_DIRECTORY)).unwrap();
    let generation = stage_broker_config(&fixture.write(&fixture.value()), &paths).unwrap();

    assert_eq!(generation.host_id(), HOST_ID);
    assert_eq!(
        generation.profile_authority_sha256(),
        PROFILE_AUTHORITY_SHA256
    );
    let snapshot = BrokerConfig::load(generation.config_path()).unwrap();
    assert_eq!(snapshot.host_id(), generation.host_id());
    assert_eq!(
        snapshot.profile_authority_sha256(),
        generation.profile_authority_sha256()
    );
}

#[test]
fn profile_authority_digest_selects_a_distinct_generation() {
    let fixture = ConfigFixture::new();
    let paths = BrokerPaths::new(fixture.root.path().join(REVISION_STATE_DIRECTORY)).unwrap();
    let first = stage_broker_config(&fixture.write(&fixture.value()), &paths).unwrap();
    let mut replacement = fixture.value();
    replacement[PROFILE_AUTHORITY_SHA256_CONFIG_FIELD] =
        json!(REPLACEMENT_PROFILE_AUTHORITY_SHA256);
    let second = stage_broker_config(&fixture.write(&replacement), &paths).unwrap();

    assert_ne!(second.revision(), first.revision());
    assert_ne!(second.config_path(), first.config_path());
    assert_eq!(
        second.profile_authority_sha256(),
        REPLACEMENT_PROFILE_AUTHORITY_SHA256
    );
}
