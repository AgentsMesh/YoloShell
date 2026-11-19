use super::*;

fn definition(schema: u16, id: &str, kind: &str) -> ProviderDefinition {
    ProviderDefinition::new(schema, id.into(), AgentKind::from_name(kind)).unwrap()
}

#[test]
fn builtin_identities_round_trip_through_validated_definitions() {
    for (id, kind) in [
        (yolo_codex_identity::PROVIDER_ID, AgentKind::Codex),
        (yolo_claude_identity::PROVIDER_ID, AgentKind::ClaudeCode),
    ] {
        let value = ProviderDefinition::new(PROVIDER_SCHEMA_VERSION, id.into(), kind.clone())
            .unwrap();
        assert_eq!(value.id(), id);
        assert_eq!(value.kind(), &kind);
    }
}

#[test]
fn unknown_provider_is_valid_and_not_reserved() {
    let value = definition(PROVIDER_SCHEMA_VERSION, "acme.provider", "acme_agent");
    assert_eq!(value.kind().as_str(), "acme_agent");
    let swapped = definition(PROVIDER_SCHEMA_VERSION, yolo_codex_identity::PROVIDER_ID, "acme_agent");
    assert_ne!(swapped.kind(), &AgentKind::Codex);
}

#[test]
fn schema_and_identity_boundaries_are_rejected() {
    for (schema, id, kind) in [
        (0, "a.provider", "agent"),
        (PROVIDER_SCHEMA_VERSION + 1, "a.provider", "agent"),
        (PROVIDER_SCHEMA_VERSION, "", "agent"),
        (PROVIDER_SCHEMA_VERSION, "provider", "agent"),
        (PROVIDER_SCHEMA_VERSION, "a..provider", "agent"),
        (PROVIDER_SCHEMA_VERSION, "a.provider", ""),
        (PROVIDER_SCHEMA_VERSION, "a.provider", "Agent"),
        (PROVIDER_SCHEMA_VERSION, "a.provider", "agent space"),
    ] {
        assert_eq!(
            ProviderDefinition::new(schema, id.into(), AgentKind::from_name(kind)),
            Err(ProviderError::Invalid)
        );
    }
}

#[test]
fn size_boundaries_are_bounded() {
    let id = format!("{}.provider", "a".repeat(MAX_PROVIDER_ID_BYTES));
    assert_eq!(
        ProviderDefinition::new(PROVIDER_SCHEMA_VERSION, id, AgentKind::Codex),
        Err(ProviderError::Invalid)
    );
    let kind = "a".repeat(MAX_PROVIDER_KIND_BYTES + 1);
    assert_eq!(
        ProviderDefinition::new(
            PROVIDER_SCHEMA_VERSION,
            "a.provider".into(),
            AgentKind::from_name(kind)
        ),
        Err(ProviderError::Invalid)
    );
}

#[test]
fn accessors_expose_validated_fields() {
    let value =
        ProviderDefinition::new(PROVIDER_SCHEMA_VERSION, yolo_codex_identity::PROVIDER_ID.into(), AgentKind::Codex)
            .unwrap();
    assert_eq!(value.schema_version(), PROVIDER_SCHEMA_VERSION);
    assert_eq!(value.id(), yolo_codex_identity::PROVIDER_ID);
    assert_eq!(value.kind(), &AgentKind::Codex);
}
