use super::runtime_intake_test_support::RoutedFixture;
use super::support::{FIRST_EVENT_SEQUENCE, Fixture};
use crate::agent::hooks::{EventSequence, HookEnvelope};
use crate::codex_hooks::{CodexHookArtifacts, CodexHookIntake, CodexOutbox};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use uuid::Uuid;

const OCCURRENCE_REPLACEMENT_FILE_NAME: &str = "occurrence-replacement";
const PRIVATE_FILE_MODE: u32 = 0o600;

#[test]
fn unrouted_intake_is_deferred_without_mutating_lifecycle_or_outbox() {
    let fixture = Fixture::new_with_prepared_retirement_writer_lock();
    let intake = intake(&fixture);
    intake
        .persist_for_test(unassigned(fixture.start(FIRST_EVENT_SEQUENCE)))
        .unwrap();

    let report = fixture.runtime.scan_unrouted_intake().unwrap();

    assert_eq!(report.deferred, 1);
    assert_eq!(
        fixture.runtime.state().unwrap().unwrap().last_sequence,
        EventSequence::ZERO
    );
    assert!(
        outbox(&fixture)
            .load()
            .unwrap()
            .pending_for_test()
            .is_empty()
    );
    assert_eq!(intake.scan().unwrap().occurrences.len(), 1);
}

#[test]
fn replaced_admission_file_defers_occurrence_before_outbox_enqueue() {
    let fixture = RoutedFixture::new();
    let intake = intake(&fixture);
    intake
        .persist_for_test(unassigned(fixture.start(FIRST_EVENT_SEQUENCE)))
        .unwrap();
    fixture.replace_admission_same_payload();

    let report = fixture.reconcile().unwrap();

    assert_eq!(report.deferred, 1);
    assert!(
        outbox(&fixture)
            .load()
            .unwrap()
            .pending_for_test()
            .is_empty()
    );
    assert_eq!(intake.scan().unwrap().occurrences.len(), 1);
}

#[test]
fn a_router_without_the_registered_route_cannot_authorize_occurrence() {
    let fixture = RoutedFixture::new();
    let intake = intake(&fixture);
    intake
        .persist_for_test(unassigned(fixture.start(FIRST_EVENT_SEQUENCE)))
        .unwrap();
    let empty = super::super::router::CodexSessionRouter::new(
        fixture.paths.clone(),
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
    )
    .unwrap();

    let report = fixture.runtime.reconcile_routed_intake(&empty).unwrap();

    assert_eq!(report.deferred, 1);
    assert!(
        outbox(&fixture)
            .load()
            .unwrap()
            .pending_for_test()
            .is_empty()
    );
    assert_eq!(intake.scan().unwrap().occurrences.len(), 1);
}

#[test]
fn provisional_route_cannot_authorize_occurrence() {
    let fixture = RoutedFixture::new();
    let intake = intake(&fixture);
    intake
        .persist_for_test(unassigned(fixture.start(FIRST_EVENT_SEQUENCE)))
        .unwrap();
    let (router, provisional) = fixture.provisional_router();

    let report = fixture.runtime.reconcile_routed_intake(&router).unwrap();

    assert_eq!(report.deferred, 1);
    assert!(
        outbox(&fixture)
            .load()
            .unwrap()
            .pending_for_test()
            .is_empty()
    );
    assert_eq!(intake.scan().unwrap().occurrences.len(), 1);
    provisional.abort().unwrap();
}

#[test]
fn stale_occurrence_file_identity_cannot_mint_a_fold_capability() {
    let fixture = RoutedFixture::new();
    let intake = intake(&fixture);
    intake
        .persist_for_test(unassigned(fixture.start(FIRST_EVENT_SEQUENCE)))
        .unwrap();
    let occurrence = intake.scan().unwrap().occurrences.pop().unwrap();
    replace_only_occurrence(&artifacts(&fixture).intake_directory());

    let result = super::super::runtime::VerifiedDurableOccurrence::authorize(
        &fixture.runtime,
        fixture.router(),
        &intake,
        &occurrence,
    );

    assert!(result.is_err());
    assert!(
        outbox(&fixture)
            .load()
            .unwrap()
            .pending_for_test()
            .is_empty()
    );
    assert_eq!(intake.scan().unwrap().occurrences.len(), 1);
}

#[test]
fn capability_rejects_changed_payload_with_the_same_identity_and_event_id() {
    let fixture = RoutedFixture::new();
    let intake = intake(&fixture);
    intake
        .persist_for_test(unassigned(fixture.start(FIRST_EVENT_SEQUENCE)))
        .unwrap();
    let occurrence = intake.scan().unwrap().occurrences.pop().unwrap();
    let proof = super::super::runtime::VerifiedDurableOccurrence::authorize(
        &fixture.runtime,
        fixture.router(),
        &intake,
        &occurrence,
    )
    .unwrap();
    let mut substituted = occurrence.envelope().clone();
    substituted.sequence = EventSequence::FIRST;
    substituted.stop_hook_active = Some(true);

    assert!(proof.verify(&fixture.runtime, &substituted).is_err());
}

fn unassigned(mut event: HookEnvelope) -> HookEnvelope {
    event.event_id = Uuid::nil();
    event.sequence = EventSequence::ZERO;
    event
}

fn intake(fixture: &Fixture) -> CodexHookIntake {
    CodexHookIntake::for_identity(artifacts(fixture).intake_directory(), &fixture.identity).unwrap()
}

fn outbox(fixture: &Fixture) -> CodexOutbox {
    CodexOutbox::for_identity(artifacts(fixture).outbox_path(), &fixture.identity).unwrap()
}

fn artifacts(fixture: &Fixture) -> CodexHookArtifacts {
    CodexHookArtifacts::new(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        fixture.identity.invocation_id,
    )
    .unwrap()
}

fn replace_only_occurrence(directory: &Path) {
    let mut entries = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    let original = entries.pop().unwrap();
    let replacement = directory.join(OCCURRENCE_REPLACEMENT_FILE_NAME);
    std::fs::write(&replacement, std::fs::read(&original).unwrap()).unwrap();
    std::fs::set_permissions(
        &replacement,
        std::fs::Permissions::from_mode(PRIVATE_FILE_MODE),
    )
    .unwrap();
    std::fs::rename(replacement, original).unwrap();
}
