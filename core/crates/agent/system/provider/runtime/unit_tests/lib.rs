use super::*;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use yolo_agent_adapter_contract::{AgentAdapter, DetectionContext};
use yolo_agent_contract::{
    AgentBinding, AgentEvidence, AgentKind, ObservedProcess, ResumePlan,
};
use yolo_agent_provider_contract::{
    AuthorityKind, InteractionSemantics, LifecycleSemantics, ProviderCapabilities,
    ProviderDescriptor, ProviderTier, ProjectionPolicy,
};

struct FixtureAdapter {
    kind: AgentKind,
}

impl AgentAdapter for FixtureAdapter {
    fn kind(&self) -> AgentKind {
        self.kind.clone()
    }
    fn matches_process(&self, _: &ObservedProcess) -> bool {
        false
    }
    fn evidence_files(&self, _: &DetectionContext) -> Result<Vec<std::path::PathBuf>> {
        Ok(Vec::new())
    }
    fn inspect_evidence(&self, _: &Path, _: &Path) -> Result<Option<AgentEvidence>> {
        Ok(None)
    }
    fn resume_plan(&self, _: &AgentBinding) -> ResumePlan {
        ResumePlan {
            executable: "fixture".into(),
            args: Vec::new(),
            environment: BTreeMap::new(),
        }
    }
    fn validate_binding(&self, _: &AgentBinding, _: &DetectionContext) -> Result<()> {
        Ok(())
    }
}

fn descriptor(kind: &str, manifest_id: &str, install: &str, conversation: &str) -> ProviderDescriptor {
    ProviderDescriptor::new(
        manifest_id.into(),
        AgentKind::Other(kind.into()),
        install.into(),
        conversation.into(),
        ProviderTier::L5,
        ProviderCapabilities::FULL,
        "Fixture".into(),
        None,
        AuthorityKind::GenericHookGeneration,
        LifecycleSemantics::StopIgnored,
        ProjectionPolicy::HOOK_AUTHORITATIVE,
    )
    .unwrap()
}

fn runtime(kind: &str, manifest_id: &str, install: &str, conversation: &str) -> ProviderRuntime {
    let descriptor = descriptor(kind, manifest_id, install, conversation);
    let adapter = Arc::new(FixtureAdapter {
        kind: descriptor.kind().clone(),
    });
    ProviderRuntime::new(descriptor, adapter).unwrap()
}

#[test]
fn typed_interaction_requires_the_native_bridge_authority() {
    let typed = runtime("fixture_a", "yolo.provider.fixture-a", "fixture-a", "fixture_a")
        .with_interaction_semantics(InteractionSemantics::Typed);
    assert!(matches!(
        ProviderRuntimeRegistry::from_runtimes([typed]),
        Err(ProviderRuntimeError::InteractionSemanticsMismatch)
    ));
    let terminal = runtime("fixture_b", "yolo.provider.fixture-b", "fixture-b", "fixture_b")
        .with_interaction_semantics(InteractionSemantics::Terminal);
    assert!(ProviderRuntimeRegistry::from_runtimes([terminal]).is_ok());
}

#[test]
fn registry_indexes_every_provider_identity() {
    let registry = ProviderRuntimeRegistry::from_runtimes([
        runtime("fixture_a", "yolo.provider.fixture-a", "fixture-a", "fixture_a"),
        runtime("fixture_b", "yolo.provider.fixture-b", "fixture-b", "fixture_b"),
    ])
    .unwrap();
    assert_eq!(registry.len(), 2);
    assert!(registry.adapter(&AgentKind::Other("fixture_a".into())).is_some());
    assert!(registry.by_manifest_id("yolo.provider.fixture-b").is_some());
    assert!(registry.by_install_token("fixture-a").is_some());
    assert!(registry
        .by_conversation_token("fixture_b")
        .is_some());
    assert!(registry.get(&AgentKind::Other("missing".into())).is_none());
    assert_eq!(registry.descriptors().count(), 2);
}

#[test]
fn duplicate_identity_is_rejected_for_each_namespace() {
    let base = runtime("fixture", "yolo.provider.fixture", "fixture", "fixture");
    assert!(ProviderRuntimeRegistry::from_runtimes([base.clone(), base.clone()]).is_err());

    let other_kind = runtime("fixture2", "yolo.provider.fixture", "fixture2", "fixture2");
    assert!(
        ProviderRuntimeRegistry::from_runtimes([base.clone(), other_kind]).is_err()
    );

    let other_manifest = runtime("fixture2", "yolo.provider.fixture2", "fixture", "fixture2");
    assert!(
        ProviderRuntimeRegistry::from_runtimes([base.clone(), other_manifest]).is_err()
    );

    let other_install = runtime("fixture2", "yolo.provider.fixture2", "fixture2", "fixture");
    assert!(
        ProviderRuntimeRegistry::from_runtimes([base, other_install]).is_err()
    );
}

#[test]
fn adapter_must_match_its_descriptor_kind() {
    let descriptor = descriptor("fixture", "yolo.provider.fixture", "fixture", "fixture");
    let adapter = Arc::new(FixtureAdapter {
        kind: AgentKind::Other("other".into()),
    });
    assert_eq!(
        ProviderRuntime::new(descriptor, adapter).err(),
        Some(ProviderRuntimeError::AdapterKindMismatch)
    );
}

mod ports;
