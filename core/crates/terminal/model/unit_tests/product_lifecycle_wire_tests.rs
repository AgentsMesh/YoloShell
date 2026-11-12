use uuid::Uuid;
use yolo_terminal_model::{TerminalDiscardRequest, TerminalLifecycleSelection, TerminalTerminateRequest};

const UNKNOWN_FIELD: &str = "unexpected";

#[test]
fn product_runtime_epoch_is_canonical_and_unknown_fields_are_rejected() {
    let epoch = Uuid::new_v4();
    let fenced = TerminalLifecycleSelection::at_runtime_epoch(Uuid::new_v4(), epoch);
    let mut encoded = serde_json::to_value(&fenced).unwrap();
    assert_eq!(
        encoded["expected_runtime_epoch"],
        serde_json::Value::String(epoch.hyphenated().to_string())
    );
    assert_eq!(
        serde_json::from_value::<TerminalLifecycleSelection>(encoded.clone()).unwrap(),
        fenced
    );
    encoded[UNKNOWN_FIELD] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<TerminalLifecycleSelection>(encoded).is_err());

    let mut noncanonical = serde_json::to_value(&fenced).unwrap();
    noncanonical["expected_runtime_epoch"] =
        serde_json::Value::String(epoch.hyphenated().to_string().to_uppercase());
    assert!(serde_json::from_value::<TerminalLifecycleSelection>(noncanonical).is_err());

    let mut discard = serde_json::to_value(TerminalDiscardRequest {
        selection: fenced.clone(),
        force: true,
    })
    .unwrap();
    assert!(serde_json::from_value::<TerminalDiscardRequest>(discard.clone()).is_ok());
    discard[UNKNOWN_FIELD] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<TerminalDiscardRequest>(discard).is_err());
    let mut terminate = serde_json::to_value(TerminalTerminateRequest {
        selection: fenced,
        force: false,
    })
    .unwrap();
    assert!(serde_json::from_value::<TerminalTerminateRequest>(terminate.clone()).is_ok());
    terminate[UNKNOWN_FIELD] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<TerminalTerminateRequest>(terminate).is_err());
}
