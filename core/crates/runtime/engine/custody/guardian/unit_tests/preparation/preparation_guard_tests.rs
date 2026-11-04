use super::install_successor_guarded;
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use crate::namespace_lease::NamespaceLease;
use crate::process::ProcessIdentityRelation;
use std::time::Duration;

#[path = "../../test_support/replacement_test_support.rs"]
mod support;
use support::Fixture;

const CONTENDER_NAMESPACE_LABEL: &str = "guardian successor lock contender";
const WRONG_GUARD_NAMESPACE_LABEL: &str = "guardian successor wrong session";
const LOCK_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn successor_installation_holds_guard_from_snapshot_through_replace() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let contender_namespace = NamespaceLease::capture(&[(
        CONTENDER_NAMESPACE_LABEL,
        &fixture.paths.session_dir(fixture.runtime.terminal_id),
    )])
    .unwrap();
    let guard = session_guard(&fixture);
    let predecessor = guard
        .namespace()
        .read_json_with_identity(&fixture.paths.pty_custody(fixture.runtime.terminal_id))
        .unwrap();
    let paths = fixture.paths.clone();
    let terminal_id = fixture.runtime.terminal_id;
    let (attempted_send, attempted_receive) = std::sync::mpsc::channel();
    let (observed_send, observed_receive) = std::sync::mpsc::channel();
    let contender = std::thread::spawn(move || {
        assert!(
            SessionDurableMutationGuard::acquire(&paths, &contender_namespace, terminal_id).is_err()
        );
        attempted_send.send(()).unwrap();
        let contender_guard = SessionDurableMutationGuard::acquire_participant(
            &paths,
            &contender_namespace,
            terminal_id,
        )
        .unwrap();
        let current = contender_guard
            .namespace()
            .read_json_with_identity(&paths.pty_custody(terminal_id))
            .unwrap();
        observed_send.send(current).unwrap();
    });
    attempted_receive.recv().unwrap();

    let preparation = install_successor_guarded(
        &guard,
        &fixture.paths,
        &predecessor.0,
        predecessor.1,
        fixture.runtime_identity,
        &fixture.request,
        &fixture.offer,
        fixture.binding(),
        None,
    )
    .unwrap();
    assert!(observed_receive.try_recv().is_err());
    let installed = fixture.current();
    assert_eq!(installed.1, preparation.custody_file_identity());
    drop(guard);

    assert_eq!(
        observed_receive.recv_timeout(LOCK_RELEASE_TIMEOUT).unwrap(),
        installed
    );
    contender.join().unwrap();
}

#[test]
fn exact_pending_retry_reuses_the_existing_guard_and_preparation() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let guard = session_guard(&fixture);
    let preparation = install_successor_guarded(
        &guard,
        &fixture.paths,
        fixture.snapshot.record(),
        fixture.snapshot.file_identity(),
        fixture.runtime_identity,
        &fixture.request,
        &fixture.offer,
        fixture.binding(),
        None,
    )
    .unwrap();
    let prepared = fixture.current();

    let retry = install_successor_guarded(
        &guard,
        &fixture.paths,
        &prepared.0,
        prepared.1,
        fixture.runtime_identity,
        &fixture.request,
        &fixture.offer,
        fixture.binding(),
        Some(preparation),
    )
    .unwrap();

    assert_eq!(retry, preparation);
    assert_eq!(fixture.current(), prepared);
}

#[test]
fn exact_pending_retry_rejects_a_guard_for_another_session() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let guard = session_guard(&fixture);
    let preparation = install_successor_guarded(
        &guard,
        &fixture.paths,
        fixture.snapshot.record(),
        fixture.snapshot.file_identity(),
        fixture.runtime_identity,
        &fixture.request,
        &fixture.offer,
        fixture.binding(),
        None,
    )
    .unwrap();
    let prepared = fixture.current();
    drop(guard);
    let wrong_session = uuid::Uuid::new_v4();
    fixture.paths.ensure_session(wrong_session).unwrap();
    let wrong_namespace = NamespaceLease::capture(&[(
        WRONG_GUARD_NAMESPACE_LABEL,
        &fixture.paths.session_dir(wrong_session),
    )])
    .unwrap();
    let wrong_guard = SessionDurableMutationGuard::acquire_participant(
        &fixture.paths,
        &wrong_namespace,
        wrong_session,
    )
    .unwrap();

    assert!(
        install_successor_guarded(
            &wrong_guard,
            &fixture.paths,
            &prepared.0,
            prepared.1,
            fixture.runtime_identity,
            &fixture.request,
            &fixture.offer,
            fixture.binding(),
            Some(preparation),
        )
        .is_err()
    );
    assert_eq!(fixture.current(), prepared);
}

fn session_guard(fixture: &Fixture) -> SessionDurableMutationGuard<'_> {
    SessionDurableMutationGuard::acquire_participant(
        &fixture.paths,
        &fixture.namespace,
        fixture.runtime.terminal_id,
    )
    .unwrap()
}
