use crate::{constants::*, fixture::Fixture};
use uuid::Uuid;
use yolo_hub_connector_protocol::{ConnectorCredential, ConnectorError, REDACTED_CREDENTIAL};

#[test]
fn credentials_are_canonical_random_uuid_secrets_with_constant_time_equality() {
    let value = ConnectorCredential::try_from(UUID_TEXT.to_owned()).unwrap();
    assert_eq!(value.as_uuid(), Uuid::parse_str(UUID_TEXT).unwrap());
    assert_eq!(String::from(value), UUID_TEXT);
    assert_eq!(
        ConnectorCredential::from_uuid(value.as_uuid()).unwrap(),
        value
    );
    assert!(value.matches(&value));
    let mut changed = *value.as_uuid().as_bytes();
    changed[0] ^= 1;
    let changed = ConnectorCredential::from_uuid(Uuid::from_bytes(changed)).unwrap();
    assert!(!value.matches(&changed));
    assert_ne!(value, changed);
    let generated = ConnectorCredential::generate();
    assert!(ConnectorCredential::from_uuid(generated.as_uuid()).is_ok());
    assert_eq!(serde_json::to_value(value).unwrap(), UUID_TEXT);
    assert_eq!(
        serde_json::from_value::<ConnectorCredential>(serde_json::json!(UUID_TEXT)).unwrap(),
        value
    );
}

#[test]
fn invalid_credentials_are_rejected_without_echoing_their_bytes() {
    for input in [
        String::new(),
        UUID_TEXT.to_uppercase(),
        UUID_TEXT.replace('-', ""),
        format!(" {UUID_TEXT}"),
        Uuid::nil().to_string(),
        NON_RANDOM_UUID.into(),
        INVALID_VARIANT_UUID.into(),
    ] {
        assert_eq!(
            ConnectorCredential::try_from(input.clone()),
            Err(ConnectorError::InvalidRequest)
        );
        assert!(serde_json::from_value::<ConnectorCredential>(serde_json::json!(input)).is_err());
    }
    assert!(serde_json::from_value::<ConnectorCredential>(serde_json::Value::Null).is_err());
}

#[test]
fn debug_redacts_credentials_even_inside_enrollment_and_activation_commands() {
    let fixture = Fixture::new();
    let secret = String::from(fixture.enrollment.credential);
    assert_eq!(
        format!("{}", fixture.enrollment.credential),
        REDACTED_CREDENTIAL
    );
    assert_eq!(
        format!("{:?}", fixture.enrollment.credential),
        REDACTED_CREDENTIAL
    );
    assert!(!format!("{:?}", fixture.enrollment).contains(&secret));
    for (request, reply) in fixture.samples() {
        assert!(!format!("{request:?} {reply:?}").contains(&secret));
    }
}
