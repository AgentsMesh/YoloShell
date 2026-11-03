use super::runtime_intake_test_support::RoutedFixture;
use super::support::FIRST_EVENT_SEQUENCE;
use crate::agent::hooks::{EventSequence, HookEnvelope};
use crate::codex_hooks::{CodexHookArtifacts, CodexHookIntake, CodexOutbox};
use crate::worker::codex_hook::router::CodexRouteRetirement;
use std::sync::Arc;
use uuid::Uuid;

const COMPLETION_TIMEOUT: std::time::Duration =
    crate::worker::codex_hook::constants::CODEX_INGRESS_BLOCKING_DRAIN_TIMEOUT;
const SINGLE_APPLIED_EVENT: usize = 1;

#[test]
fn begin_replay_does_not_deadlock_behind_reconcile_lifecycle_wait() {
    let fixture = Arc::new(RoutedFixture::new());
    persist_start(&fixture);
    let lifecycle = fixture.runtime.lock_lifecycle_anyhow().unwrap();
    let gate = fixture.runtime.gate_next_intake_authorization_for_test();
    let (finished, completion) = std::sync::mpsc::channel();
    let worker_fixture = Arc::clone(&fixture);
    let worker = std::thread::spawn(move || {
        finished.send(worker_fixture.reconcile()).unwrap();
    });
    gate.wait_until_entered();

    let replay = fixture.replay_lease();
    assert!(replay.is_replay());
    replay.settle().unwrap();
    drop(lifecycle);
    gate.release();

    let report = completion
        .recv_timeout(COMPLETION_TIMEOUT)
        .unwrap()
        .unwrap();
    worker.join().unwrap();
    assert_eq!(report.applied, SINGLE_APPLIED_EVENT);
}

#[test]
fn retirement_waits_for_an_authorized_fold_to_release_its_route_lease() {
    let fixture = Arc::new(RoutedFixture::new());
    persist_start(&fixture);
    let entry = fixture.router().resolve(&fixture.identity).unwrap();
    let gate = fixture.runtime.gate_next_intake_fold_for_test();
    let (finished, completion) = std::sync::mpsc::channel();
    let worker_fixture = Arc::clone(&fixture);
    let worker = std::thread::spawn(move || {
        finished.send(worker_fixture.reconcile()).unwrap();
    });
    gate.wait_until_entered();

    assert_eq!(
        fixture.router().retire(&entry).unwrap(),
        CodexRouteRetirement::Deferred
    );
    gate.release();

    let report = completion
        .recv_timeout(COMPLETION_TIMEOUT)
        .unwrap()
        .unwrap();
    worker.join().unwrap();
    assert_eq!(report.applied, SINGLE_APPLIED_EVENT);
    assert!(fixture.router().resolve(&fixture.identity).is_err());
}

fn persist_start(fixture: &RoutedFixture) {
    let intake = intake(fixture);
    let occurrence = intake
        .persist_occurrence_for_test(unassigned(fixture.start(FIRST_EVENT_SEQUENCE)))
        .unwrap();
    let identity = intake
        .find_exact_occurrence(&occurrence)
        .unwrap()
        .unwrap()
        .durable_identity()
        .unwrap();
    CodexOutbox::for_identity(fixture.runtime.outbox_path(), &fixture.identity)
        .unwrap()
        .enqueue_admitted_for_test(occurrence, Some(&identity), |_, _| Ok(()), |_, _, _| Ok(()))
        .unwrap();
}

fn unassigned(mut event: HookEnvelope) -> HookEnvelope {
    event.event_id = Uuid::nil();
    event.sequence = EventSequence::ZERO;
    event
}

fn intake(fixture: &RoutedFixture) -> CodexHookIntake {
    let artifacts = CodexHookArtifacts::new(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        fixture.identity.invocation_id,
    )
    .unwrap();
    CodexHookIntake::for_identity(artifacts.intake_directory(), &fixture.identity).unwrap()
}
