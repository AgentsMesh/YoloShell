use super::runtime_intake_test_support::RoutedFixture;
use super::runtime_retirement_intake_test_support::{
    assigned, capture_seal, persisted_occurrence, replace_same_contents,
};
use crate::codex_hooks::{CodexHookArtifacts, CodexOutbox};
use crate::worker::codex_hook::runtime::VerifiedDurableOccurrence;
use std::sync::Arc;
use std::time::Duration;

const WRITER_LOCK_REPLACEMENT_FILE_NAME: &str = "retirement-writer-lock-replacement";
const FENCE_OBSERVATION_TIMEOUT: Duration = Duration::from_millis(100);
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(2);
const SINGLE_APPLIED_EVENT: usize = 1;

#[test]
fn sealed_invocation_cannot_mint_an_active_occurrence_authority() {
    let fixture = RoutedFixture::new();
    let (intake, occurrence) = persisted_occurrence(&fixture);
    let entry = fixture.router().resolve(&fixture.identity).unwrap();
    let admission = entry.admission().authority();
    let writer = admission.acquire_retirement_writer_fence().unwrap();
    writer
        .publish_seal(&capture_seal(admission), admission)
        .unwrap();
    drop(writer);

    let result = VerifiedDurableOccurrence::authorize(
        &fixture.runtime,
        fixture.router(),
        &intake,
        &occurrence,
    );

    assert!(result.is_err());
}

#[test]
fn replaced_writer_lock_invalidates_an_active_occurrence_authority() {
    let fixture = RoutedFixture::new();
    let (intake, occurrence) = persisted_occurrence(&fixture);
    let proof = VerifiedDurableOccurrence::authorize(
        &fixture.runtime,
        fixture.router(),
        &intake,
        &occurrence,
    )
    .unwrap();
    let artifacts = CodexHookArtifacts::new(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        fixture.identity.invocation_id,
    )
    .unwrap();
    replace_same_contents(
        &artifacts.retirement_writer_lock_path_for_test(),
        WRITER_LOCK_REPLACEMENT_FILE_NAME,
    );

    assert!(
        proof
            .verify(&fixture.runtime, &assigned(&occurrence))
            .is_err()
    );
}

#[test]
fn seal_waits_until_the_authorized_fold_and_outbox_ack_finish() {
    let fixture = Arc::new(RoutedFixture::new());
    let (_, occurrence) = persisted_occurrence(&fixture);
    let occurrence_identity = occurrence.durable_identity().unwrap();
    CodexOutbox::for_identity(fixture.runtime.outbox_path(), &fixture.identity)
        .unwrap()
        .enqueue_admitted_for_test(
            occurrence.envelope().clone(),
            Some(&occurrence_identity),
            |_, _| Ok(()),
            |_, _, _| Ok(()),
        )
        .unwrap();
    let gate = fixture.runtime.gate_next_intake_fold_for_test();
    let (reconcile_tx, reconcile_rx) = std::sync::mpsc::channel();
    let worker_fixture = Arc::clone(&fixture);
    let worker = std::thread::spawn(move || {
        reconcile_tx.send(worker_fixture.reconcile()).unwrap();
    });
    gate.wait_until_entered();
    let entry = fixture.router().resolve(&fixture.identity).unwrap();
    let admission = entry.admission().authority().clone();
    let seal = capture_seal(&admission);
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (sealed_tx, sealed_rx) = std::sync::mpsc::channel();
    let sealer = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        let writer = admission.acquire_retirement_writer_fence().unwrap();
        let sealed = writer.publish_seal(&seal, &admission).is_ok();
        sealed_tx.send(sealed).unwrap();
    });
    started_rx.recv_timeout(COMPLETION_TIMEOUT).unwrap();

    assert!(
        matches!(
            sealed_rx.recv_timeout(FENCE_OBSERVATION_TIMEOUT),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ),
        "seal acquired the active occurrence writer fence"
    );
    gate.release();

    let report = reconcile_rx
        .recv_timeout(COMPLETION_TIMEOUT)
        .unwrap()
        .unwrap();
    assert_eq!(report.applied, SINGLE_APPLIED_EVENT);
    assert!(sealed_rx.recv_timeout(COMPLETION_TIMEOUT).unwrap());
    worker.join().unwrap();
    sealer.join().unwrap();
    let outbox =
        CodexOutbox::for_identity(fixture.runtime.outbox_path(), &fixture.identity).unwrap();
    assert!(outbox.load().unwrap().pending_for_test().is_empty());
}
