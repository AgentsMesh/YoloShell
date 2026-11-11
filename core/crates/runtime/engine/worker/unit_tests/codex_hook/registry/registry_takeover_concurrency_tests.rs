use super::support::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Barrier};

type Runtime = super::super::runtime::CodexHookRuntime;
type Registry = super::super::authority_registry::CodexAuthorityRegistry;

const CONCURRENT_SUCCESSOR_COUNT: usize = 3;
const SINGLE_CURRENT_AUTHORITY: usize = 1;
const SUCCESSOR_BUILD_PANIC: &str = "injected successor build panic";

#[test]
fn concurrent_successors_leave_exactly_one_current_authority() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let barrier = Arc::new(Barrier::new(CONCURRENT_SUCCESSOR_COUNT));
    let runtimes = std::thread::scope(|scope| {
        let handles = (0..CONCURRENT_SUCCESSOR_COUNT)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let registry = Arc::clone(&registry);
                let fixture = &fixture;
                scope.spawn(move || {
                    barrier.wait();
                    build(fixture, &registry).unwrap()
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });

    assert_eq!(
        runtimes.iter().filter(|runtime| is_active(runtime)).count(),
        SINGLE_CURRENT_AUTHORITY
    );

    let final_runtime = build(&fixture, &registry).unwrap();
    assert!(is_active(&final_runtime));
    assert!(runtimes.iter().all(|runtime| is_superseded(runtime)));
}

#[test]
fn panicking_successor_build_contains_predecessor_and_poisons_takeover() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let predecessor = build(&fixture, &registry).unwrap();
    super::super::runtime::set_before_initial_publish_hook(|| panic!("{SUCCESSOR_BUILD_PANIC}"));

    let result = catch_unwind(AssertUnwindSafe(|| build(&fixture, &registry)));

    assert!(result.is_err());
    assert_eq!(
        predecessor.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::AuthorityLost
    );
    assert!(build(&fixture, &registry).is_err());
    assert!(is_active(&fresh(&fixture).unwrap()));
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

fn fresh(fixture: &Fixture) -> anyhow::Result<Arc<Runtime>> {
    super::super::runtime::CodexHookRuntime::new_test(
        &fixture.paths,
        Fixture::terminal_id(),
        Fixture::runtime_epoch(),
        fixture.shell,
        fixture.identity.clone(),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        fixture.process_source(),
    )
    .map(|(runtime, _)| runtime)
}

fn is_active(runtime: &Runtime) -> bool {
    matches!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Active(_)
    )
}

fn is_superseded(runtime: &Runtime) -> bool {
    runtime.subscribe_authority().borrow().clone()
        == super::super::publication::CodexAuthorityView::Superseded
}
