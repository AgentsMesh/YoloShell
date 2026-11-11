use super::runtime_intake_test_support::RoutedFixture;
use super::support::{FIRST_EVENT_SEQUENCE, Fixture};
use crate::agent::hooks::{EventSequence, HookEnvelope, ProcessState};
use crate::codex_hooks::{CodexHookArtifacts, CodexHookIntake, CodexOutbox};
use uuid::Uuid;

#[path = "../../../test_support/unit/codex_hook/runtime/runtime_intake_api.rs"]
mod fixture_api;
use fixture_api::*;

const SINGLE_OCCURRENCE: usize = 1;

#[test]
fn raw_only_occurrence_is_deferred_without_mutating_state_or_outbox() {
    let fixture = RoutedFixture::new();
    let (intake, _) = persist_raw(&fixture, fixture.start(FIRST_EVENT_SEQUENCE));

    let report = fixture.reconcile().unwrap();

    assert_eq!(report.applied, usize::default());
    assert_eq!(report.deferred, SINGLE_OCCURRENCE);
    assert_eq!(intake.scan().unwrap().occurrences.len(), SINGLE_OCCURRENCE);
    let state = outbox(&fixture).load().unwrap();
    assert_eq!(state.acknowledged_sequence(), EventSequence::ZERO.0);
    assert!(state.pending_for_test().is_empty());
    assert_eq!(
        fixture.runtime.state().unwrap().unwrap().last_sequence,
        EventSequence::ZERO
    );
}

#[test]
fn authoritative_occurrence_is_folded_acknowledged_and_completed() {
    let fixture = RoutedFixture::new();
    let (intake, occurrence) = persist_raw(&fixture, fixture.start(FIRST_EVENT_SEQUENCE));
    enqueue_durable(&fixture, &intake, occurrence);

    let report = fixture.reconcile().unwrap();

    assert_eq!(report.applied, SINGLE_OCCURRENCE);
    assert_eq!(report.deferred, usize::default());
    assert!(intake.scan().unwrap().occurrences.is_empty());
    let state = outbox(&fixture).load().unwrap();
    assert_eq!(state.acknowledged_sequence(), FIRST_EVENT_SEQUENCE);
    assert!(state.pending_for_test().is_empty());
    let lifecycle = fixture.runtime.state().unwrap().unwrap();
    assert_eq!(lifecycle.last_sequence, EventSequence::FIRST);
    assert_eq!(lifecycle.process, ProcessState::Alive);
}

#[test]
fn raw_only_occurrence_does_not_block_an_authoritative_tail() {
    let fixture = RoutedFixture::new();
    let (intake, _) = persist_raw(&fixture, fixture.prompt(FIRST_EVENT_SEQUENCE));
    let occurrence = intake
        .persist_occurrence_for_test(unassigned(fixture.start(FIRST_EVENT_SEQUENCE)))
        .unwrap();
    enqueue_durable(&fixture, &intake, occurrence);

    let report = fixture.reconcile().unwrap();

    assert_eq!(report.deferred, SINGLE_OCCURRENCE);
    assert_eq!(report.applied, SINGLE_OCCURRENCE);
    assert_eq!(intake.scan().unwrap().occurrences.len(), SINGLE_OCCURRENCE);
    let state = outbox(&fixture).load().unwrap();
    assert_eq!(state.acknowledged_sequence(), FIRST_EVENT_SEQUENCE);
    assert!(state.pending_for_test().is_empty());
    assert_eq!(
        fixture.runtime.state().unwrap().unwrap().last_sequence,
        EventSequence::FIRST
    );
}

#[test]
fn cold_replay_completes_occurrence_left_after_outbox_commit() {
    let fixture = RoutedFixture::new();
    let (intake, occurrence) = persist_raw(&fixture, fixture.start(FIRST_EVENT_SEQUENCE));
    enqueue_durable(&fixture, &intake, occurrence);

    let (successor, restored) = fixture.restart_runtime();
    let router = fixture.router_for(&successor);
    let report = successor.reconcile_routed_intake(&router).unwrap();

    assert_eq!(restored.applied, SINGLE_OCCURRENCE);
    assert_eq!(report.replayed, SINGLE_OCCURRENCE);
    assert!(intake.scan().unwrap().occurrences.is_empty());
    assert_eq!(
        outbox(&fixture).load().unwrap().acknowledged_sequence(),
        FIRST_EVENT_SEQUENCE
    );
}

#[test]
fn replay_completes_occurrence_left_after_snapshot_commit() {
    let fixture = RoutedFixture::new();
    let (intake, occurrence) = persist_raw(&fixture, fixture.start(FIRST_EVENT_SEQUENCE));
    let event = enqueue_durable(&fixture, &intake, occurrence);
    fixture.runtime.ingest(event, fixture.process.pid).unwrap();

    let report = fixture.reconcile().unwrap();

    assert_eq!(report.replayed, SINGLE_OCCURRENCE);
    assert_eq!(report.deferred, usize::default());
    assert!(intake.scan().unwrap().occurrences.is_empty());
    assert_eq!(
        outbox(&fixture).load().unwrap().acknowledged_sequence(),
        FIRST_EVENT_SEQUENCE
    );
}

#[test]
fn same_event_id_with_different_payload_cannot_replace_exact_authority() {
    let fixture = RoutedFixture::new();
    let (intake, occurrence) = persist_raw(&fixture, fixture.start(FIRST_EVENT_SEQUENCE));
    let committed = enqueue_durable(&fixture, &intake, occurrence.clone());
    let mut conflict = occurrence;
    conflict.stop_hook_active = Some(true);

    assert!(outbox(&fixture).enqueue(conflict).is_err());
    let state = outbox(&fixture).load().unwrap();
    assert_eq!(state.pending_for_test().len(), SINGLE_OCCURRENCE);
    assert_eq!(state.pending_for_test().front(), Some(&committed));

    let report = fixture.reconcile().unwrap();
    assert_eq!(report.applied, SINGLE_OCCURRENCE);
    assert!(intake.scan().unwrap().occurrences.is_empty());
}
#[test]
fn indeterminate_routed_ack_latches_storage_authority() {
    let fixture = RoutedFixture::new();
    let (intake, occurrence) = persist_raw(&fixture, fixture.start(FIRST_EVENT_SEQUENCE));
    enqueue_durable(&fixture, &intake, occurrence);
    let outbox = outbox(&fixture);
    let path = outbox.path().to_path_buf();
    crate::codex_hooks::set_outbox_commit_hook(path.clone(), || {
        crate::paths::test_support::fail_next_directory_sync();
    });
    crate::codex_hooks::set_outbox_reproof_hook(path.clone(), move || {
        std::fs::remove_file(path).unwrap();
    });

    let report = fixture.reconcile().unwrap();

    assert_eq!(report.deferred, SINGLE_OCCURRENCE);
    assert_eq!(intake.scan().unwrap().occurrences.len(), SINGLE_OCCURRENCE);
    assert!(fixture.runtime.storage_failed().unwrap());
    assert_eq!(fixture.runtime.state().unwrap(), None);
    assert_eq!(
        fixture.runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::StorageUnavailable
    );
    let follow_up = fixture.reconcile().unwrap();
    assert_eq!(follow_up.applied, usize::default());
    assert_eq!(follow_up.replayed, usize::default());
    assert_eq!(follow_up.deferred, SINGLE_OCCURRENCE);
}
