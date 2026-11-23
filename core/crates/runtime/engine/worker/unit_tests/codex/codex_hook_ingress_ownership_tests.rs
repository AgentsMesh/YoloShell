use crate::worker::tests::*;
#[test]
fn worker_states_own_isolated_codex_ingress_registries() {
    let first = Fixture::new();
    let second = Fixture::new();
    let first_registry = first.state.codex_hook_ingress.registry_probe();
    let second_registry = second.state.codex_hook_ingress.registry_probe();
    assert!(!first_registry.shares_allocation(&second_registry));

    first.state.shutdown.notify_waiters();
    assert!(first_registry.is_alive());
    assert!(second_registry.is_alive());

    drop(first);
    assert!(!first_registry.is_alive());
    assert!(second_registry.is_alive());

    drop(second);
    assert!(!second_registry.is_alive());
}
