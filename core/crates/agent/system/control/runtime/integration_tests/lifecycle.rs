use crate::fixture::*;
use uuid::Uuid;
use yolo_agent_control_runtime::*;
use yolo_startup_checkpoint_contract::{StartupCheckpointEvidence, StartupPhase, StartupStatus};

#[test]
fn interrupted_request_stays_unknown_and_completed_receipt_survives_reopen() {
    let fixture = Fixture::new();
    let mut owner = fixture.initialize();
    let interrupted = Uuid::new_v4();
    assert_eq!(
        owner
            .lookup(&fixture.runtime, interrupted, digest())
            .unwrap(),
        None
    );
    drop(reserve(&mut owner, &fixture.runtime, interrupted));
    assert_eq!(
        owner
            .lookup(&fixture.runtime, interrupted, digest())
            .unwrap(),
        Some(Receipt::Unknown)
    );
    let delivered = Uuid::new_v4();
    let reservation = reserve(&mut owner, &fixture.runtime, delivered);
    assert_eq!(
        owner
            .complete(&fixture.runtime, reservation, Completion::Delivered)
            .unwrap(),
        Receipt::Delivered
    );
    drop(owner);
    let mut reopened = fixture.reopen(&fixture.runtime).unwrap();
    assert!(matches!(
        reopened
            .reserve(&fixture.runtime, interrupted, digest())
            .unwrap(),
        ReservationResult::Existing(Receipt::Unknown)
    ));
    assert!(matches!(
        reopened
            .reserve(&fixture.runtime, delivered, digest())
            .unwrap(),
        ReservationResult::Existing(Receipt::Delivered)
    ));
}

#[test]
fn normal_startup_retirement_does_not_invalidate_existing_history() {
    let fixture = Fixture::new();
    let mut owner = fixture.initialize();
    let request = Uuid::new_v4();
    drop(reserve(&mut owner, &fixture.runtime, request));
    let id = fixture.runtime.terminal_id;
    fixture
        .namespace()
        .remove_file(&fixture.paths.worker_launch(id))
        .unwrap();
    fixture.write(&fixture.paths.runtime_record(id), &fixture.runtime);
    let checkpoint_path = fixture.paths.worker_startup_checkpoint(id);
    let mut checkpoint: StartupCheckpointEvidence = fixture.read(&checkpoint_path);
    checkpoint.status = StartupStatus::RuntimeActivated;
    checkpoint.phase = StartupPhase::Completed;
    fixture.write(&checkpoint_path, &checkpoint);
    assert!(fixture.proof().is_err());
    owner.verify(&fixture.runtime).unwrap();
    drop(owner);
    let reopened = fixture.reopen(&fixture.runtime).unwrap();
    assert_eq!(
        reopened
            .lookup(&fixture.runtime, request, digest())
            .unwrap(),
        Some(Receipt::Unknown)
    );
}

#[test]
fn request_conflicts_do_not_poison_the_valid_owner() {
    let fixture = Fixture::new();
    let mut owner = fixture.initialize();
    let request = Uuid::new_v4();
    let reservation = reserve(&mut owner, &fixture.runtime, request);
    assert_eq!(
        owner.lookup(&fixture.runtime, request, other_digest()),
        Err(JournalError::Conflict)
    );
    assert!(matches!(
        owner.reserve(&fixture.runtime, request, other_digest()),
        Err(JournalError::Conflict)
    ));
    assert_eq!(
        owner
            .complete(&fixture.runtime, reservation, Completion::NotDelivered)
            .unwrap(),
        Receipt::NotDelivered
    );
}
