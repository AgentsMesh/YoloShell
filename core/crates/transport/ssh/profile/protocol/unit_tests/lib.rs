#[path = "../test_support/constants.rs"]
mod constants;
use constants::*;
use yolo_ssh_profile_protocol::{MAX_PROFILE_ID_BYTES, PROFILE_REVISION_HEX_BYTES, SshProfileRef};

fn revision() -> String {
    REVISION_CHARACTER.repeat(PROFILE_REVISION_HEX_BYTES)
}

#[test]
fn references_round_trip_without_credentials_or_connection_identity() {
    assert!(SshProfileRef::new(PROFILE, NUMERIC_REVISION_CHARACTER.repeat(PROFILE_REVISION_HEX_BYTES)).is_ok());
    let profile = SshProfileRef::new(PROFILE, revision()).unwrap();
    let encoded = serde_json::to_value(&profile).unwrap();
    assert_eq!(profile.profile_id(), PROFILE);
    assert_eq!(profile.authority_sha256(), revision());
    assert_eq!(
        encoded.as_object().unwrap().len(),
        [PROFILE_FIELD, REVISION_FIELD].len()
    );
    assert_eq!(
        serde_json::from_value::<SshProfileRef>(encoded).unwrap(),
        profile
    );
}

#[test]
fn invalid_paths_and_noncanonical_revisions_are_rejected() {
    for profile in INVALID_PROFILES {
        assert!(SshProfileRef::new(*profile, revision()).is_err());
    }
    assert!(SshProfileRef::new(PROFILE.repeat(MAX_PROFILE_ID_BYTES), revision()).is_err());
    for invalid in [String::new(), revision().to_uppercase(), PROFILE.into()] {
        assert!(SshProfileRef::new(PROFILE, invalid).is_err());
    }
    assert!(
        SshProfileRef::new(REVISION_CHARACTER.repeat(MAX_PROFILE_ID_BYTES), revision()).is_ok()
    );
}

#[test]
fn decoding_rejects_missing_unknown_duplicate_and_invalid_fields() {
    let value = serde_json::to_value(SshProfileRef::new(PROFILE, revision()).unwrap()).unwrap();
    for field in [PROFILE_FIELD, REVISION_FIELD] {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<SshProfileRef>(missing).is_err());
        let mut invalid = value.clone();
        invalid[field] = serde_json::Value::String(String::new());
        assert!(serde_json::from_value::<SshProfileRef>(invalid).is_err());
    }
    let mut extra = value.clone();
    extra[UNKNOWN_FIELD] = serde_json::Value::Null;
    assert!(serde_json::from_value::<SshProfileRef>(extra).is_err());
    let json = serde_json::to_string(&value).unwrap();
    let duplicate = format!("{{\"{PROFILE_FIELD}\":\"{PROFILE}\",{}", &json[1..]);
    assert!(serde_json::from_str::<SshProfileRef>(&duplicate).is_err());
}
