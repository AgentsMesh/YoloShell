use crate::support::*;

fn rejected(value: serde_json::Value) {
    assert!(serde_json::from_value::<TerminalPresentation>(value).is_err());
}

#[test]
fn current_fields_are_required_but_nullable_and_roundtrip() {
    for original in [TerminalPresentation::default(), presentation()] {
        let value = serde_json::to_value(&original).unwrap();
        assert_eq!(
            serde_json::from_value::<TerminalPresentation>(value.clone()).unwrap(),
            original
        );
        for key in [TITLE_FIELD, SEQUENCE_FIELD, NOTIFICATION_FIELD] {
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove(key);
            rejected(missing);
        }
        let mut unknown = value;
        unknown[UNKNOWN_FIELD] = true.into();
        rejected(unknown);
    }
}

#[test]
fn untrusted_metadata_cannot_inject_controls_or_exceed_limits() {
    for title in [
        ESCAPE_INJECTION.to_owned(),
        BIDI_INJECTION.to_owned(),
        UNICODE_TEXT.repeat(MAX_TITLE_BYTES),
    ] {
        let mut value = serde_json::to_value(TerminalPresentation::default()).unwrap();
        value[TITLE_FIELD] = title.into();
        rejected(value);
    }
    for (field, limit) in [
        (TITLE_FIELD, MAX_NOTIFICATION_TITLE_BYTES),
        (BODY_FIELD, MAX_NOTIFICATION_BODY_BYTES),
    ] {
        for text in [
            ESCAPE_INJECTION.to_owned(),
            BIDI_INJECTION.to_owned(),
            UNICODE_TEXT.repeat(limit),
        ] {
            let mut value = serde_json::to_value(presentation()).unwrap();
            value[NOTIFICATION_FIELD][field] = text.into();
            rejected(value);
        }
    }
}

#[test]
fn inconsistent_notification_shape_is_rejected() {
    let value = serde_json::to_value(presentation()).unwrap();
    for key in [TITLE_FIELD, BODY_FIELD, KIND_FIELD] {
        let mut missing = value.clone();
        missing[NOTIFICATION_FIELD]
            .as_object_mut()
            .unwrap()
            .remove(key);
        rejected(missing);
    }
    let mut bell = value.clone();
    bell[NOTIFICATION_FIELD][KIND_FIELD] = BELL_KIND.into();
    rejected(bell);
    let mut zero = value.clone();
    zero[SEQUENCE_FIELD] = u64::default().into();
    rejected(zero);
    let mut missing = value;
    missing[NOTIFICATION_FIELD] = serde_json::Value::Null;
    rejected(missing);
    let mut unknown = serde_json::to_value(presentation()).unwrap();
    unknown[NOTIFICATION_FIELD][UNKNOWN_FIELD] = true.into();
    rejected(unknown);
}
