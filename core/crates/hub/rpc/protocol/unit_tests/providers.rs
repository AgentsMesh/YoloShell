use yolo_hub_rpc_protocol::{
    ProviderFeatures, ProviderInteraction, ProviderSummary,
};
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn provider_features_round_trip_and_missing_features_default() {
    let summary = ProviderSummary::new("codex", "Codex", true)
        .unwrap()
        .with_features(ProviderFeatures {
            conversation: true,
            submit_prompt: true,
            interrupt: true,
            interaction: ProviderInteraction::Terminal,
        });
    let json = serde_json::to_value(&summary).unwrap();
    assert_eq!(json["features"]["conversation"], true);
    assert_eq!(json["features"]["interaction"], "terminal");
    let old: ProviderSummary = serde_json::from_value(serde_json::json!({
        "provider_id": "old",
        "display_name": "Old",
        "available": false
    }))
    .unwrap();
    assert!(!old.features.conversation);
    assert_eq!(old.features.interaction, ProviderInteraction::None);
}

#[test]
fn provider_summary_accepts_stable_ids_and_display_names() {
    let value = ProviderSummary::new("alpha", "Alpha", false).unwrap();
    assert_eq!(value.provider_id, "alpha");
    assert_eq!(value.display_name, "Alpha");
    assert!(!value.available);
    assert!(ProviderSummary::new("beta-code", "Beta", true).is_ok());
}

#[test]
fn provider_summary_rejects_unsafe_identity_or_display() {
    assert!(matches!(
        ProviderSummary::new("", "Codex", false),
        Err(PublicError::InvalidMetadata)
    ));
    assert!(matches!(
        ProviderSummary::new("codex/../../x", "Codex", false),
        Err(PublicError::InvalidMetadata)
    ));
    assert!(matches!(
        ProviderSummary::new("codex", "bad\nname", false),
        Err(PublicError::InvalidMetadata)
    ));
    assert!(matches!(
        ProviderSummary::new("codex", "   ", false),
        Err(PublicError::InvalidMetadata)
    ));
}
