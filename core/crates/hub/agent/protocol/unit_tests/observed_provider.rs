use yolo_hub_agent_protocol::*;

use crate::constants::*;
use crate::fixtures::*;

#[test]
fn provider_metadata_is_opaque_bounded_and_supports_other_connectors() {
    let provider = ObservedProvider::new(
        PROVIDER_KIND.into(),
        Some(PROVIDER_VERSION.into()),
        Some(NATIVE_REFERENCE.into()),
    )
    .unwrap();
    assert_eq!(provider.kind(), PROVIDER_KIND);
    assert_eq!(provider.version(), Some(PROVIDER_VERSION));
    assert_eq!(provider.native_session_reference(), Some(NATIVE_REFERENCE));
    assert_eq!(round_trip(&provider), provider);
    let absent = ObservedProvider::new(PROVIDER_KIND.into(), None, None).unwrap();
    assert_eq!(absent.version(), None);
    assert_eq!(absent.native_session_reference(), None);
    assert_eq!(round_trip(&absent), absent);
}

#[test]
fn provider_validation_checks_each_field_and_exact_limits() {
    for kind in [
        String::new(),
        INVALID_PROVIDER.into(),
        TEXT_CHARACTER.repeat(MAX_PROVIDER_KIND_BYTES + SEQUENCE_STEP as usize),
    ] {
        assert_eq!(
            ObservedProvider::new(kind, None, None),
            Err(ProtocolError::InvalidProvider)
        );
    }
    let max_kind = TEXT_CHARACTER.repeat(MAX_PROVIDER_KIND_BYTES);
    let max_version = TEXT_CHARACTER.repeat(MAX_PROVIDER_VERSION_BYTES);
    let max_reference = TEXT_CHARACTER.repeat(MAX_NATIVE_SESSION_REFERENCE_BYTES);
    assert!(ObservedProvider::new(max_kind, Some(max_version), Some(max_reference)).is_ok());
    for invalid in [
        String::new(),
        "line\nbreak".into(),
        TEXT_CHARACTER.repeat(MAX_PROVIDER_VERSION_BYTES + SEQUENCE_STEP as usize),
    ] {
        assert_eq!(
            ObservedProvider::new(PROVIDER_KIND.into(), Some(invalid), None),
            Err(ProtocolError::InvalidProvider)
        );
    }
    for invalid in [
        String::new(),
        "nul\0byte".into(),
        TEXT_CHARACTER.repeat(MAX_NATIVE_SESSION_REFERENCE_BYTES + SEQUENCE_STEP as usize),
    ] {
        assert_eq!(
            ObservedProvider::new(PROVIDER_KIND.into(), None, Some(invalid)),
            Err(ProtocolError::InvalidProvider)
        );
    }
    let mut wire =
        serde_json::to_value(ObservedProvider::new(PROVIDER_KIND.into(), None, None).unwrap())
            .unwrap();
    wire["kind"] = INVALID_PROVIDER.into();
    assert!(serde_json::from_value::<ObservedProvider>(wire).is_err());
}

#[test]
fn provider_display_name_is_optional_bounded_and_catalog_owned() {
    let display_name = "Provider Name";
    let provider = ObservedProvider::new(PROVIDER_KIND.into(), None, None)
        .unwrap()
        .with_display_name(Some(display_name.into()))
        .unwrap();
    assert_eq!(provider.display_name(), Some(display_name));
    assert_eq!(round_trip(&provider), provider);
    let absent = ObservedProvider::new(PROVIDER_KIND.into(), None, None)
        .unwrap()
        .with_display_name(None)
        .unwrap();
    assert_eq!(absent.display_name(), None);
    for invalid in [
        String::new(),
        "line\nbreak".into(),
        TEXT_CHARACTER.repeat(MAX_PROVIDER_DISPLAY_NAME_BYTES + SEQUENCE_STEP as usize),
    ] {
        assert_eq!(
            ObservedProvider::new(PROVIDER_KIND.into(), None, None)
                .unwrap()
                .with_display_name(Some(invalid)),
            Err(ProtocolError::InvalidProvider)
        );
    }
    let mut legacy = serde_json::json!({
        "kind": PROVIDER_KIND,
        "version": null
    });
    legacy[NATIVE_REFERENCE_FIELD] = serde_json::Value::Null;
    assert_eq!(
        serde_json::from_value::<ObservedProvider>(legacy)
            .unwrap()
            .display_name(),
        None
    );
}
