use super::*;
use yolo_agent_contract::AgentKind;

fn descriptor(
    capabilities: ProviderCapabilities,
    target_tier: ProviderTier,
    projection_policy: ProjectionPolicy,
) -> Result<ProviderDescriptor, ProviderDescriptorError> {
    ProviderDescriptor::new(
        "yolo.provider.fixture".into(),
        AgentKind::Other("fixture".into()),
        "fixture".into(),
        "fixture".into(),
        target_tier,
        capabilities,
        "Fixture".into(),
        None,
        AuthorityKind::GenericHookGeneration,
        LifecycleSemantics::StopIgnored,
        projection_policy,
    )
}

#[test]
fn full_capabilities_guarantee_l5_and_serialize_round_trip() {
    let value = descriptor(
        ProviderCapabilities::FULL,
        ProviderTier::L5,
        ProjectionPolicy::HOOK_AUTHORITATIVE,
    )
    .unwrap();
    assert_eq!(value.guaranteed_floor(), ProviderTier::L5);
    assert_eq!(value.authority_kind(), AuthorityKind::GenericHookGeneration);
    assert_eq!(value.kind(), &AgentKind::Other("fixture".into()));
    let encoded = serde_json::to_vec(&value).unwrap();
    let decoded: ProviderDescriptor = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn tiers_require_all_lower_capabilities() {
    let l2 = ProviderCapabilities {
        detect: true,
        resume_exact: true,
        lifecycle: true,
        ..ProviderCapabilities::UNSUPPORTED
    };
    assert_eq!(l2.floor(), ProviderTier::L2);
    assert!(descriptor(l2, ProviderTier::L2, ProjectionPolicy::HOOK_AUTHORITATIVE).is_ok());
    assert!(descriptor(l2, ProviderTier::L3, ProjectionPolicy::HOOK_AUTHORITATIVE).is_err());

    let l4 = ProviderCapabilities {
        conversation: true,
        submit: true,
        interrupt: true,
        ..l2
    };
    assert_eq!(l4.floor(), ProviderTier::L4);
    assert!(descriptor(l4, ProviderTier::L4, ProjectionPolicy::HOOK_AUTHORITATIVE).is_ok());
    assert!(descriptor(l4, ProviderTier::L5, ProjectionPolicy::HOOK_AUTHORITATIVE).is_err());
}

#[test]
fn invalid_identity_tokens_and_claims_fail_closed() {
    let invalid_id = ProviderDescriptor::new(
        "fixture".into(),
        AgentKind::Other("fixture".into()),
        "fixture".into(),
        "fixture".into(),
        ProviderTier::L0,
        ProviderCapabilities::UNSUPPORTED,
        "Fixture".into(),
        None,
        AuthorityKind::None,
        LifecycleSemantics::StopIgnored,
        ProjectionPolicy::PROCESS_OBSERVER,
    );
    assert!(invalid_id.is_err());

    let invalid_kind = ProviderDescriptor::new(
        "yolo.provider.fixture".into(),
        AgentKind::Other("Fixture!".into()),
        "fixture".into(),
        "fixture".into(),
        ProviderTier::L0,
        ProviderCapabilities::UNSUPPORTED,
        "Fixture".into(),
        None,
        AuthorityKind::None,
        LifecycleSemantics::StopIgnored,
        ProjectionPolicy::PROCESS_OBSERVER,
    );
    assert!(invalid_kind.is_err());

    let invalid_token = ProviderDescriptor::new(
        "yolo.provider.fixture".into(),
        AgentKind::Other("fixture".into()),
        "Fixture!".into(),
        "fixture".into(),
        ProviderTier::L0,
        ProviderCapabilities::UNSUPPORTED,
        "Fixture".into(),
        None,
        AuthorityKind::None,
        LifecycleSemantics::StopIgnored,
        ProjectionPolicy::PROCESS_OBSERVER,
    );
    assert!(invalid_token.is_err());
}

#[test]
fn projection_and_recovery_claims_require_the_owning_capabilities() {
    let no_lifecycle = ProviderCapabilities {
        detect: true,
        resume_exact: true,
        ..ProviderCapabilities::UNSUPPORTED
    };
    assert!(
        descriptor(
            no_lifecycle,
            ProviderTier::L1,
            ProjectionPolicy::HOOK_AUTHORITATIVE
        )
        .is_err()
    );

    let recovery_without_authority = ProviderDescriptor::new(
        "yolo.provider.fixture".into(),
        AgentKind::Other("fixture".into()),
        "fixture".into(),
        "fixture".into(),
        ProviderTier::L5,
        ProviderCapabilities::FULL,
        "Fixture".into(),
        None,
        AuthorityKind::None,
        LifecycleSemantics::StopIgnored,
        ProjectionPolicy::HOOK_AUTHORITATIVE,
    );
    assert!(recovery_without_authority.is_err());
}

#[test]
fn projection_policies_default_to_process_observer_and_follow_descriptors() {
    let hook = descriptor(
        ProviderCapabilities::FULL,
        ProviderTier::L5,
        ProjectionPolicy::HOOK_AUTHORITATIVE,
    )
    .unwrap();
    let policies = ProjectionPolicies::from_descriptors([&hook]);
    assert_eq!(
        policies.policy(&AgentKind::Other("fixture".into())),
        ProjectionPolicy::HOOK_AUTHORITATIVE
    );
    assert_eq!(
        policies.policy(&AgentKind::Other("unknown".into())),
        ProjectionPolicy::PROCESS_OBSERVER
    );
    assert!(!ProjectionPolicies::none().policy(&AgentKind::Codex).hook_authoritative);
}

#[test]
fn launch_policies_expose_stable_entries() {
    let mut policies = LaunchPolicies::none();
    policies.insert(AgentKind::Codex, "codex");
    policies.insert(AgentKind::Other("fixture".into()), "fixture-bin");
    let entries = policies.entries().collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|(kind, command)| {
        *kind == &AgentKind::Codex && *command == "codex"
    }));
    assert_eq!(policies.command("fixture"), Some("fixture-bin"));
}

#[test]
fn unsupported_schema_is_rejected_on_decode() {
    let value = descriptor(
        ProviderCapabilities::FULL,
        ProviderTier::L5,
        ProjectionPolicy::HOOK_AUTHORITATIVE,
    )
    .unwrap();
    let mut encoded: serde_json::Value = serde_json::to_value(&value).unwrap();
    encoded[fields::SCHEMA_VERSION] = serde_json::json!(99);
    assert!(serde_json::from_value::<ProviderDescriptor>(encoded).is_err());
}
