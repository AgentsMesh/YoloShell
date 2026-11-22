use yolo_agent_contract::AgentKind;
use yolo_plugin_provider::*;

fn builtin_definitions() -> Vec<ProviderDefinition> {
    [
        (yolo_codex_identity::PROVIDER_ID, AgentKind::Codex),
        (yolo_claude_identity::PROVIDER_ID, AgentKind::ClaudeCode),
        (
            yolo_qwen_identity::PROVIDER_ID,
            AgentKind::Other(yolo_qwen_identity::KIND.into()),
        ),
        (
            yolo_gemini_identity::PROVIDER_ID,
            AgentKind::Other(yolo_gemini_identity::KIND.into()),
        ),
        (
            yolo_grok_identity::PROVIDER_ID,
            AgentKind::Other(yolo_grok_identity::KIND.into()),
        ),
    ]
    .into_iter()
    .map(|(id, kind)| ProviderDefinition::new(PROVIDER_SCHEMA_VERSION, id.into(), kind).unwrap())
    .collect()
}

#[test]
fn definitions_round_trip_as_strict_json() {
    for value in builtin_definitions() {
        let encoded = serde_json::to_value(&value).unwrap();
        let decoded: ProviderDefinition = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, value);
    }
}

#[test]
fn unknown_fields_and_invalid_wire_values_fail_closed() {
    let valid = serde_json::json!({
        yolo_plugin_provider::SCHEMA_VERSION_FIELD: PROVIDER_SCHEMA_VERSION,
        "id": "acme.provider",
        "kind": "acme_agent"
    });
    let mut unknown = valid.clone();
    unknown["extra"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ProviderDefinition>(unknown).is_err());
    for field in [
        serde_json::json!({yolo_plugin_provider::SCHEMA_VERSION_FIELD: 0, "id": "a.provider", "kind": "agent"}),
        serde_json::json!({yolo_plugin_provider::SCHEMA_VERSION_FIELD: 1, "id": "a/provider", "kind": "agent"}),
        serde_json::json!({yolo_plugin_provider::SCHEMA_VERSION_FIELD: 1, "id": "a.provider", "kind": "Agent"}),
    ] {
        assert!(serde_json::from_value::<ProviderDefinition>(field).is_err());
    }
}

#[test]
fn builtin_catalog_is_unique_and_uses_stable_kinds() {
    let values = builtin_definitions();
    let ids = values
        .iter()
        .map(ProviderDefinition::id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), values.len());
    assert_eq!(AgentKind::Codex.as_str(), "codex");
    assert_eq!(
        values.last().unwrap().kind().as_str(),
        yolo_grok_identity::KIND
    );
    assert_eq!(ids.len(), 5);
}
