use super::*;

const STATE_FILE: &str = "contract-outbox.json";
const REQUIRED_FIELDS: &[&str] = &[
    stringify!(owner),
    stringify!(acknowledged_events),
    stringify!(pending_occurrence_witnesses),
    stringify!(occurrence_receipts),
    stringify!(durable_rejections),
];

#[test]
fn current_empty_state_has_explicit_null_owner_and_empty_collections() {
    let original = CodexOutboxState::default();
    let value = serde_json::to_value(&original).unwrap();
    assert_eq!(value[stringify!(owner)], serde_json::Value::Null);
    assert_eq!(
        serde_json::from_value::<CodexOutboxState>(value).unwrap(),
        original
    );
}

#[test]
fn persisted_outbox_rejects_each_missing_current_field_without_rewriting() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(STATE_FILE);
    let original = serde_json::to_value(CodexOutboxState::default()).unwrap();
    for field in REQUIRED_FIELDS {
        let mut value = original.clone();
        value.as_object_mut().unwrap().remove(*field);
        assert!(
            serde_json::from_value::<CodexOutboxState>(value.clone()).is_err(),
            "{field}"
        );
        crate::paths::atomic_write_json(&path, &value).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(CodexOutbox::new(&path).is_err(), "{field}");
        assert_eq!(std::fs::read(&path).unwrap(), before, "{field}");
    }
}
