use super::support::*;
use std::sync::Arc;

type Runtime = super::super::runtime::CodexHookRuntime;
type Registry = super::super::authority_registry::CodexAuthorityRegistry;
use super::super::authority_registry::CodexRegistryPoisonTarget;

#[test]
fn dead_current_control_fails_closed() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let runtime = build(&fixture, &registry).unwrap();
    let _retained = registry
        .retain_dead_current_for_test(
            Fixture::terminal_id(),
            Fixture::runtime_epoch(),
            fixture.identity.invocation_id,
        )
        .unwrap();

    assert!(build(&fixture, &registry).is_err());
    assert_active(&runtime);
}

#[test]
fn every_registry_mutex_poison_fails_closed() {
    let fixture = Fixture::new();
    let targets = [
        CodexRegistryPoisonTarget::Registry,
        CodexRegistryPoisonTarget::Takeover,
        CodexRegistryPoisonTarget::Current,
    ];

    for target in targets {
        let registry = Arc::new(Registry::default());
        let runtime = build(&fixture, &registry).unwrap();
        registry.poison_for_test(
            target,
            Fixture::terminal_id(),
            Fixture::runtime_epoch(),
            fixture.identity.invocation_id,
        );

        assert!(build(&fixture, &registry).is_err());
        assert_active(&runtime);
    }
}

fn build(fixture: &Fixture, registry: &Arc<Registry>) -> anyhow::Result<Arc<Runtime>> {
    super::super::runtime::CodexHookRuntime::new_test_in_registry(
        &fixture.paths,
        Arc::clone(registry),
        Fixture::terminal_id(),
        Fixture::runtime_epoch(),
        fixture.shell,
        fixture.identity.clone(),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        fixture.process_source(),
    )
    .map(|(runtime, _)| runtime)
}

fn assert_active(runtime: &Runtime) {
    assert!(matches!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Active(_)
    ));
}
