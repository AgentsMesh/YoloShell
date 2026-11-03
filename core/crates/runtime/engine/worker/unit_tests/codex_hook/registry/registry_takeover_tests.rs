use super::support::*;
use std::sync::Arc;

type Runtime = super::super::runtime::CodexHookRuntime;
type Registry = super::super::authority_registry::CodexAuthorityRegistry;

#[test]
fn empty_outbox_takeover_supersedes_predecessor_before_return() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let first = build(&fixture, &registry).unwrap();

    let second = build(&fixture, &registry).unwrap();

    assert_superseded(&first);
    assert_active(&second);
    assert!(matches!(
        first.ingest(fixture.start(FIRST_EVENT_SEQUENCE), fixture.process.pid),
        Err(super::super::error::CodexHookRejection::Superseded)
    ));
    first.contain_authority_panic();
    assert_superseded(&first);
}

#[test]
fn stale_registration_drops_never_remove_the_current_runtime() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let first = build(&fixture, &registry).unwrap();
    let second = build(&fixture, &registry).unwrap();
    let third = build(&fixture, &registry).unwrap();
    assert_superseded(&first);
    assert_superseded(&second);
    assert_active(&third);

    drop(first);
    drop(second);
    let fourth = build(&fixture, &registry).unwrap();

    assert_superseded(&third);
    assert_active(&fourth);
}

#[test]
fn current_lineage_advances_independently_from_registration_claim() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let first = build(&fixture, &registry).unwrap();
    let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
    first.ingest(event.clone(), fixture.process.pid).unwrap();

    let second = build(&fixture, &registry).unwrap();

    assert_superseded(&first);
    assert_eq!(
        second.state().unwrap().unwrap().last_sequence,
        event.sequence
    );
}

#[test]
fn exhausted_authority_can_be_taken_over_in_the_same_worker() {
    let fixture = Fixture::new();
    super::exhaustion_support::install_exhausted_active_authority(&fixture);
    let registry = Arc::new(Registry::default());
    let first = build(&fixture, &registry).unwrap();
    assert_exhausted(&first);

    let second = build(&fixture, &registry).unwrap();

    assert_superseded(&first);
    assert_exhausted(&second);
}

#[test]
fn terminal_authority_requires_a_fresh_worker_registry() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let runtime = build(&fixture, &registry).unwrap();
    runtime.contain_authority_panic();
    assert!(matches!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::StorageUnavailable
    ));

    assert!(build(&fixture, &registry).is_err());
    let recovered = super::super::runtime::CodexHookRuntime::new_test(
        &fixture.paths,
        Fixture::terminal_id(),
        Fixture::runtime_epoch(),
        fixture.shell,
        fixture.identity.clone(),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        fixture.process_source(),
    )
    .unwrap()
    .0;
    assert_active(&recovered);
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

fn assert_exhausted(runtime: &Runtime) {
    assert!(matches!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Exhausted(_)
    ));
}

fn assert_superseded(runtime: &Runtime) {
    assert_eq!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Superseded
    );
}
