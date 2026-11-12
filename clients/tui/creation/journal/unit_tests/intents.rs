use super::{records::document, support::*};
use crate::{CreationIntent, MAX_PENDING_CREATIONS};
use yolo_hub_identity::HubId;

#[test]
fn intent_wire_is_exact_and_debug_does_not_expose_request_secrets() {
    let record = pending(HubId::new());
    let intent = intent(&record);
    let encoded = serde_json::to_value(&intent).unwrap();
    assert_eq!(
        serde_json::from_value::<CreationIntent>(encoded.clone()).unwrap(),
        intent
    );
    let mut injected = encoded;
    injected[UNKNOWN_FIELD] = true.into();
    assert!(serde_json::from_value::<CreationIntent>(injected).is_err());
    let debug = format!("{intent:?}");
    assert!(debug.contains(&record.request.fields().request_id.to_string()));
    for private in [
        record.request.fields().creation_token.to_string(),
        REQUEST_DIRECTORY.into(),
    ] {
        assert!(!debug.contains(&private));
    }
}

#[test]
fn document_rejects_intent_aliases_across_both_record_kinds_and_origins() {
    let original = pending(HubId::new());
    for changed in std::iter::once(original.clone()).chain(conflicting_confirmations(&original)) {
        let mut value = document();
        value.pending = vec![original.clone()];
        value.intents = vec![intent(&changed)];
        assert!(value.validate(value.entry_hub, value.directory).is_err());
        value.pending.clear();
        value.intents.push(intent(&original));
        assert!(value.validate(value.entry_hub, value.directory).is_err());
    }
}

#[test]
fn intent_and_confirmed_records_share_one_capacity_limit() {
    let mut value = document();
    value.intents = (value.pending.len()..MAX_PENDING_CREATIONS)
        .map(|_| intent(&pending(HubId::new())))
        .collect();
    value.validate(value.entry_hub, value.directory).unwrap();
    value.intents.push(intent(&pending(HubId::new())));
    assert!(value.validate(value.entry_hub, value.directory).is_err());
}
