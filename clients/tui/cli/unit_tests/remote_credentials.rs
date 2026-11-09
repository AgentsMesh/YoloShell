use super::remote_fixture::{constants::*, keys, *};
use crate::selection::Selection;
use std::fs;

#[test]
fn valid_private_profile_and_separate_keys_select_only_the_named_remote_identity() {
    let fixture = Fixture::new();
    let Selection::Explicit(options) = fixture.select().unwrap() else {
        panic!()
    };
    assert_eq!(options.hub, fixture.profile.hub.hub_id);
    assert_eq!(
        options.endpoint.client_id(),
        Some(fixture.profile.client_id)
    );
    assert!(options.endpoint.local_socket().is_none());
    assert!(options.endpoint.validate().is_ok());
    fixture.private_error(&anyhow::anyhow!(format!("{:?}", options.endpoint)));
}

#[test]
fn standard_account_ecdsa_is_accepted_but_a_non_ed25519_hub_client_key_is_rejected() {
    let fixture = Fixture::new();
    let account = keys::encode(&keys::p256());
    fs::write(fixture.path(Input::AccountKey), account.as_bytes()).unwrap();
    assert!(matches!(fixture.select().unwrap(), Selection::Explicit(_)));
    fs::write(fixture.path(Input::ClientKey), account.as_bytes()).unwrap();
    fixture.rejected();
}

#[test]
fn encrypted_account_or_client_keys_are_rejected_without_disclosing_the_password() {
    for input in KEYS {
        let fixture = Fixture::new();
        let key = keys::encrypted(&keys::ed25519());
        assert!(key.is_encrypted());
        fs::write(fixture.path(input), keys::encode(&key).as_bytes()).unwrap();
        fixture.rejected();
    }
}

#[test]
fn malformed_or_non_utf8_key_content_cannot_fall_back_or_leak_credential_bytes() {
    for input in KEYS {
        for bytes in [INVALID_KEY, INVALID_UTF8] {
            let fixture = Fixture::new();
            fs::write(fixture.path(input), bytes).unwrap();
            fixture.rejected();
        }
    }
}

#[test]
fn malformed_profiles_and_invalid_identity_fields_never_select_local_startup() {
    let fixture = Fixture::new();
    fs::write(fixture.path(Input::Profile), INVALID_KEY).unwrap();
    fixture.rejected();
    let original = serde_json::to_value(&fixture.profile).unwrap();
    for mutate in [
        (|value: &mut serde_json::Value| value[FIELD_SCHEMA] = INVALID_SCHEMA.into())
            as fn(&mut serde_json::Value),
        |value| value[FIELD_CLIENT] = uuid::Uuid::nil().to_string().into(),
        |value| {
            let length = value[FIELD_HUB][FIELD_PUBLIC_KEY].as_array().unwrap().len();
            value[FIELD_HUB][FIELD_PUBLIC_KEY] = serde_json::json!(vec![u8::MIN; length]);
        },
    ] {
        let mut candidate = original.clone();
        mutate(&mut candidate);
        fs::write(
            fixture.path(Input::Profile),
            serde_json::to_vec(&candidate).unwrap(),
        )
        .unwrap();
        fixture.rejected();
    }
}
