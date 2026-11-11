use super::exhaustion_support::install_penultimate_active_authority;
use super::exhaustion_tests::final_event;
use super::runtime_intake_test_support::RoutedFixture;
use super::runtime_retirement_intake_test_support::RetirementResources;
use super::support::{Fixture, INVALID_PROCESS_START_DELTA, UNASSIGNED_EVENT_SEQUENCE};
use crate::agent::hooks::{ProcessIdentity, ProcessState, HookProviderKind, Reducer};
use crate::codex_hooks::{CodexOutbox, CodexRetirementDisposition};
use std::sync::Arc;
use std::time::Duration;

const LOCK_OBSERVATION_TIMEOUT: Duration = Duration::from_millis(100);
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(2);

macro_rules! prepare {
    ($runtime:expr, $resources:expr) => {
        $runtime
            .prepare_retirement(
                $resources.writer(),
                $resources.seal(),
                $resources.admission(),
            )
            .unwrap()
    };
}

#[test]
fn active_pending_suffix_is_replayed_exited_and_drained() {
    let fixture = RoutedFixture::new();
    let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
    let resources = RetirementResources::new(&fixture);

    let prepared = prepare!(fixture.runtime, resources);
    let reducer = restore(&fixture, &prepared.snapshot);

    assert_eq!(reducer.state().process, ProcessState::Exited);
    assert_eq!(reducer.state().last_sequence, event.sequence);
    assert_eq!(
        prepared.record.disposition_for_test(),
        CodexRetirementDisposition::Abandoned
    );
    drop(prepared);
    let outbox = outbox(&fixture, &fixture.runtime);
    assert!(outbox.pending_for_test().is_empty());
    assert_eq!(outbox.acknowledged_sequence(), event.sequence.0);
}

#[test]
fn already_exited_snapshot_is_retired_without_another_revision() {
    let fixture = RoutedFixture::new();
    fixture
        .runtime
        .observe_process_exit(observed(&fixture))
        .unwrap();
    let before = fixture.runtime.state().unwrap().unwrap();
    let resources = RetirementResources::new(&fixture);

    let prepared = prepare!(fixture.runtime, resources);
    let reducer = restore(&fixture, &prepared.snapshot);

    assert_eq!(reducer.state(), &before);
    assert_eq!(
        prepared.record.disposition_for_test(),
        CodexRetirementDisposition::Abandoned
    );
}

#[test]
fn revoked_snapshot_keeps_its_canonical_terminal_disposition() {
    let fixture = RoutedFixture::new();
    let mismatched = ProcessIdentity::new(
        fixture.process.pid,
        fixture
            .process
            .start_time
            .checked_add(INVALID_PROCESS_START_DELTA)
            .unwrap(),
    );
    assert!(fixture.runtime.observe_process_exit(mismatched).is_err());
    let resources = RetirementResources::new(&fixture);

    let prepared = prepare!(fixture.runtime, resources);
    let reducer = restore(&fixture, &prepared.snapshot);

    assert!(reducer.state().revoked);
    assert_eq!(
        prepared.record.disposition_for_test(),
        CodexRetirementDisposition::Revoked
    );
}

#[test]
fn exhausted_snapshot_acknowledges_its_durable_terminal_event() {
    let fixture = RoutedFixture::new();
    install_penultimate_active_authority(&fixture);
    let event = fixture.enqueue(final_event(&fixture, UNASSIGNED_EVENT_SEQUENCE));
    let (runtime, _) = fixture.restart_runtime();
    let router = fixture.router_for(&runtime);
    let resources = RetirementResources::for_route(&router, &fixture.identity);

    let prepared = prepare!(runtime, resources);
    let reducer = restore(&fixture, &prepared.snapshot);

    assert!(reducer.authority_exhausted());
    assert_eq!(
        prepared.record.disposition_for_test(),
        CodexRetirementDisposition::AuthorityExhausted
    );
    drop(prepared);
    let outbox = outbox(&fixture, &runtime);
    assert!(outbox.pending_for_test().is_empty());
    assert_eq!(outbox.acknowledged_sequence(), event.sequence.0);
}

#[test]
fn prepared_proofs_release_lifecycle_and_retain_terminal_locks() {
    let fixture = Arc::new(RoutedFixture::new());
    let resources = RetirementResources::new(&fixture);
    let outbox =
        CodexOutbox::for_identity(fixture.runtime.outbox_path(), &fixture.identity).unwrap();
    let prepared = prepare!(fixture.runtime, resources);
    let lifecycle_runtime = Arc::clone(&fixture.runtime);
    let (lifecycle_tx, lifecycle_rx) = std::sync::mpsc::channel();
    let lifecycle = std::thread::spawn(move || {
        let acquired = lifecycle_runtime.lock_lifecycle_anyhow().is_ok();
        lifecycle_tx.send(acquired).unwrap();
    });
    assert!(lifecycle_rx.recv_timeout(COMPLETION_TIMEOUT).unwrap());
    lifecycle.join().unwrap();
    let snapshot_runtime = Arc::clone(&fixture.runtime);
    let (state_tx, state_rx) = std::sync::mpsc::channel();
    let state = std::thread::spawn(move || state_tx.send(snapshot_runtime.state()).unwrap());
    let event = fixture.prompt(UNASSIGNED_EVENT_SEQUENCE);
    let (enqueue_tx, enqueue_rx) = std::sync::mpsc::channel();
    let enqueue = std::thread::spawn(move || enqueue_tx.send(outbox.enqueue(event)).unwrap());

    assert!(matches!(
        state_rx.recv_timeout(LOCK_OBSERVATION_TIMEOUT),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    assert!(matches!(
        enqueue_rx.recv_timeout(LOCK_OBSERVATION_TIMEOUT),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    drop(prepared);
    assert!(state_rx.recv_timeout(COMPLETION_TIMEOUT).unwrap().is_ok());
    assert!(enqueue_rx.recv_timeout(COMPLETION_TIMEOUT).unwrap().is_ok());
    state.join().unwrap();
    enqueue.join().unwrap();
}

fn restore(fixture: &Fixture, snapshot: &crate::agent::hooks::ReducerSnapshot) -> Reducer {
    Reducer::restore(
        snapshot.clone(),
        fixture.identity.clone(),
        HookProviderKind::Codex,
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
    )
    .unwrap()
}

fn observed(fixture: &Fixture) -> ProcessIdentity {
    ProcessIdentity::new(fixture.process.pid, fixture.process.start_time)
}

fn outbox(
    fixture: &Fixture,
    runtime: &crate::worker::codex_hook::runtime::CodexHookRuntime,
) -> crate::codex_hooks::CodexOutboxState {
    CodexOutbox::for_identity(runtime.outbox_path(), &fixture.identity)
        .unwrap()
        .load()
        .unwrap()
}
