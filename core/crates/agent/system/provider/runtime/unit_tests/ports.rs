use super::super::*;
use super::runtime;
use anyhow::Result;
use std::sync::Arc;
use yolo_agent_contract::{AgentBinding, AgentKind};
use yolo_agent_provider_contract::{InterruptDelivery, PromptSubmit};

struct FixtureProbe;

impl TurnEndProbe for FixtureProbe {
    fn read_turn_end(&self, _: &AgentBinding) -> Result<Option<TurnEndRecord>> {
        Ok(Some(TurnEndRecord {
            turn_id: "turn".into(),
            kind: TurnEndKind::Completed,
        }))
    }
}

#[test]
fn turn_end_probe_is_optional_per_provider() {
    let kind = AgentKind::Other("fixture".into());
    let with_probe = runtime("fixture", "yolo.provider.fixture", "fixture", "fixture")
        .with_turn_end_probe(Arc::new(FixtureProbe));
    let without = runtime("other", "yolo.provider.other", "other", "other");
    let registry = ProviderRuntimeRegistry::from_runtimes([with_probe, without]).unwrap();
    assert!(registry.turn_end_probe(&kind).is_some());
    assert!(
        registry
            .turn_end_probe(&AgentKind::Other("other".into()))
            .is_none()
    );
}

const SETTLE: std::time::Duration = std::time::Duration::from_millis(100);

#[test]
fn prompt_submit_is_atomic_unless_a_provider_declares_a_gap() {
    let kind = AgentKind::Other("fixture".into());
    let deferred = runtime("fixture", "yolo.provider.fixture", "fixture", "fixture")
        .with_prompt_submit(PromptSubmit::DeferredEnter { gap: SETTLE });
    let atomic = runtime("other", "yolo.provider.other", "other", "other");
    let registry = ProviderRuntimeRegistry::from_runtimes([deferred, atomic]).unwrap();
    assert_eq!(
        registry.get(&kind).unwrap().prompt_submit().enter_gap(),
        SETTLE
    );
    assert_eq!(
        registry
            .get(&AgentKind::Other("other".into()))
            .unwrap()
            .prompt_submit(),
        PromptSubmit::Atomic
    );
}

#[test]
fn interrupt_delivery_is_unsupported_unless_a_provider_declares_it() {
    const ESCAPE: &[u8] = b"\x1b";
    let keyed = runtime("fixture", "yolo.provider.fixture", "fixture", "fixture")
        .with_interrupt_delivery(InterruptDelivery::Key(ESCAPE));
    let signal = runtime("other", "yolo.provider.other", "other", "other")
        .with_interrupt_delivery(InterruptDelivery::Signal);
    let unsupported = runtime("third", "yolo.provider.third", "third", "third");
    let registry = ProviderRuntimeRegistry::from_runtimes([keyed, signal, unsupported]).unwrap();
    let of = |kind: &str| {
        registry
            .get(&AgentKind::Other(kind.into()))
            .unwrap()
            .interrupt_delivery()
    };
    assert_eq!(of("fixture"), InterruptDelivery::Key(ESCAPE));
    assert_eq!(of("other"), InterruptDelivery::Signal);
    assert_eq!(of("third"), InterruptDelivery::Unsupported);
}

#[test]
fn empty_registry_has_no_provider() {
    let registry = ProviderRuntimeRegistry::default();
    assert!(registry.is_empty());
    assert!(registry.adapters().next().is_none());
    assert!(registry.turn_end_probe(&AgentKind::Codex).is_none());
    assert!(registry.by_install_token("none").is_none());
}
