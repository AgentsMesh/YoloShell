use super::support::*;
use std::sync::Arc;

type Runtime = super::super::runtime::CodexHookRuntime;
type Registry = super::super::authority_registry::CodexAuthorityRegistry;
const EPOCH_HOME_DIRECTORY: &str = "epoch-home";

#[test]
fn registry_keys_isolate_session_epoch_and_invocation() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let first = build_at(
        &fixture,
        &registry,
        &fixture.paths,
        Fixture::terminal_id(),
        Fixture::runtime_epoch(),
        fixture.identity.clone(),
    );

    let other_session = Fixture::another_session();
    fixture.paths.ensure_session(other_session).unwrap();
    let mut session_identity = fixture.identity.clone();
    session_identity.terminal_id = other_session;
    let session_runtime = build_at(
        &fixture,
        &registry,
        &fixture.paths,
        other_session,
        Fixture::runtime_epoch(),
        session_identity,
    );

    let epoch_home = tempfile::tempdir().unwrap();
    let epoch_paths = crate::paths::Paths::new(epoch_home.path().join(EPOCH_HOME_DIRECTORY));
    epoch_paths.ensure_session(Fixture::terminal_id()).unwrap();
    let other_epoch = Fixture::another_runtime_epoch();
    let mut epoch_identity = fixture.identity.clone();
    epoch_identity.runtime_epoch = other_epoch;
    let epoch_runtime = build_at(
        &fixture,
        &registry,
        &epoch_paths,
        Fixture::terminal_id(),
        other_epoch,
        epoch_identity,
    );

    let mut invocation_identity = fixture.identity.clone();
    invocation_identity.invocation_id = Fixture::another_invocation();
    invocation_identity.hook_token = Fixture::other_event_id();
    let invocation_runtime = build_at(
        &fixture,
        &registry,
        &fixture.paths,
        Fixture::terminal_id(),
        Fixture::runtime_epoch(),
        invocation_identity,
    );

    for runtime in [
        &first,
        &session_runtime,
        &epoch_runtime,
        &invocation_runtime,
    ] {
        assert_active(runtime);
    }

    let successor = build_at(
        &fixture,
        &registry,
        &fixture.paths,
        Fixture::terminal_id(),
        Fixture::runtime_epoch(),
        fixture.identity.clone(),
    );

    assert_superseded(&first);
    assert_active(&successor);
    for runtime in [&session_runtime, &epoch_runtime, &invocation_runtime] {
        assert_active(runtime);
    }
}

fn build_at(
    fixture: &Fixture,
    registry: &Arc<Registry>,
    paths: &crate::paths::Paths,
    terminal_id: uuid::Uuid,
    runtime_epoch: uuid::Uuid,
    identity: crate::agent::hooks::HookIdentity,
) -> Arc<Runtime> {
    super::super::runtime::CodexHookRuntime::new_test_in_registry(
        paths,
        Arc::clone(registry),
        terminal_id,
        runtime_epoch,
        fixture.shell,
        identity,
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        fixture.process_source(),
    )
    .unwrap()
    .0
}

fn assert_active(runtime: &Runtime) {
    assert!(matches!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Active(_)
    ));
}

fn assert_superseded(runtime: &Runtime) {
    assert_eq!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Superseded
    );
}
