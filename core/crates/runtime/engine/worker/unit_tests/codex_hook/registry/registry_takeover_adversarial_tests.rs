use super::support::*;
use crate::local_unix::LocalUnixStream;
use std::sync::Arc;

type Runtime = super::super::runtime::CodexHookRuntime;
type Registry = super::super::authority_registry::CodexAuthorityRegistry;

#[test]
fn failed_initial_publication_contains_predecessor_authority() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let first = build(&fixture, &registry).unwrap();
    let path = first.snapshot_path();
    super::super::runtime::set_before_initial_publish_hook(move || {
        crate::codex_hooks::replace_snapshot_same_content(&path);
    });

    assert!(build(&fixture, &registry).is_err());

    assert_eq!(
        first.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::AuthorityLost
    );
    assert!(build(&fixture, &registry).is_err());
    let recovered = fresh(&fixture).unwrap();
    assert!(matches!(
        recovered.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Active(_)
    ));
}

#[test]
fn snapshot_substitution_after_lineage_check_contains_predecessor_authority() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let first = build(&fixture, &registry).unwrap();
    crate::codex_hooks::inject_snapshot_claim_fault(
        &first.snapshot_path(),
        crate::codex_hooks::SnapshotClaimStage::BeforeCommit,
        crate::codex_hooks::SnapshotClaimFault::TargetReplaced,
    );

    assert!(build(&fixture, &registry).is_err());

    assert_eq!(
        first.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::AuthorityLost
    );
    assert!(build(&fixture, &registry).is_err());
    assert!(matches!(
        fresh(&fixture)
            .unwrap()
            .subscribe_authority()
            .borrow()
            .clone(),
        super::super::publication::CodexAuthorityView::Active(_)
    ));
}

#[tokio::test]
async fn takeover_waits_for_accepted_ack_settlement() {
    let fixture = Fixture::new();
    let registry = Arc::new(Registry::default());
    let first = build(&fixture, &registry).unwrap();
    let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
    let response_gate = first.gate_next_response_for_test();
    let (server, mut client) = LocalUnixStream::pair().unwrap();
    let handler = tokio::spawn(super::super::handler::handle(server, Arc::clone(&first)));
    write_bytes_frame(&mut client, &yolo_capnp_codec::encode(&event).unwrap()).await;
    response_gate.wait_until_entered().await;

    let fixture_paths = fixture.paths.clone();
    let shell = fixture.shell;
    let identity = fixture.identity.clone();
    let source = fixture.process_source();
    let successor_registry = Arc::clone(&registry);
    let mut successor = tokio::task::spawn_blocking(move || {
        super::super::runtime::CodexHookRuntime::new_test_in_registry(
            &fixture_paths,
            successor_registry,
            Fixture::terminal_id(),
            Fixture::runtime_epoch(),
            shell,
            identity,
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            source,
        )
    });
    assert!(
        tokio::time::timeout(
            super::super::constants::CODEX_SOCKET_PROBE_TIMEOUT,
            &mut successor,
        )
        .await
        .is_err()
    );

    response_gate.release();
    let ack = read_ack(&mut client).await;
    let (_, response) = handler.await.unwrap().unwrap().into_parts();
    response.unwrap();
    let second = successor.await.unwrap().unwrap().0;

    assert!(ack.accepted);
    assert_eq!(
        second.state().unwrap().unwrap().last_sequence,
        event.sequence
    );
    assert_eq!(
        first.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Superseded
    );
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
