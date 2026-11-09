use super::recovery_event_support::*;
use super::recovery_test_support::*;
use crate::agent::AgentState;

#[test]
fn every_queued_event_prefix_recovers_to_independently_observed_facts() {
    for count in SESSION_START_COUNT..=ALL_EVENTS_COUNT {
        let fixture = RecoveryFixture::new(count);
        let _guards = fixture.gone_guards();
        let recovered = fixture.load().unwrap().unwrap();
        assert_eq!(recovered.state, expected_state(count));
        let expected_cwd = if count >= CWD_CHANGED_COUNT {
            &fixture.final_cwd
        } else {
            &fixture.base.begin.cwd
        };
        assert_eq!(recovered.cwd, expected_cwd.to_string_lossy());
        assert!(
            crate::claude_hooks::outbox::pending_recovery(&fixture.outbox, fixture.credentials)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            fixture
                .base
                .store
                .publication_generation_in_namespace(&fixture.base.lease)
                .unwrap(),
            2
        );
    }
}

#[derive(Clone, Copy)]
enum CrashPoint {
    AfterEnqueue,
    AfterAuthorityApply,
    AfterBindingWrite,
    AfterFinalMarker,
    AfterOutboxAck,
}

#[test]
fn every_recovery_persistence_boundary_is_idempotently_reentered() {
    for point in [
        CrashPoint::AfterEnqueue,
        CrashPoint::AfterAuthorityApply,
        CrashPoint::AfterBindingWrite,
        CrashPoint::AfterFinalMarker,
        CrashPoint::AfterOutboxAck,
    ] {
        let fixture = RecoveryFixture::new(ALL_EVENTS_COUNT);
        let _guards = fixture.gone_guards();
        advance(&fixture, point);
        let generation_before = fixture
            .base
            .store
            .publication_generation_in_namespace(&fixture.base.lease)
            .unwrap();
        let recovered = fixture.load().unwrap().unwrap();
        assert_eq!(recovered.state, AgentState::WaitingForInput);
        assert_eq!(recovered.cwd, fixture.final_cwd.to_string_lossy());
        let recovered_again = fixture.load().unwrap().unwrap();
        assert_eq!(recovered_again.state, AgentState::WaitingForInput);
        assert_eq!(
            fixture
                .base
                .store
                .publication_generation_in_namespace(&fixture.base.lease)
                .unwrap(),
            generation_before.max(2)
        );
    }
}

#[test]
fn same_sequence_with_altered_payload_is_rejected_without_acknowledgement() {
    let fixture = RecoveryFixture::new(SESSION_START_COUNT);
    let _guards = fixture.gone_guards();
    let mut altered = fixture.recovery_events();
    let GenerationEvent::SessionStart { event_id, .. } = &mut altered[0].event else {
        unreachable!();
    };
    *event_id = uuid::Uuid::new_v4();
    fixture
        .base
        .store
        .apply_recovery_in_namespace(
            &fixture.base.lease,
            fixture.authority.authentication(),
            &fixture.authority,
            &altered,
        )
        .unwrap();
    assert!(fixture.load().is_err());
    assert!(
        crate::claude_hooks::outbox::pending_recovery(&fixture.outbox, fixture.credentials)
            .unwrap()
            .is_some()
    );
    assert!(!fixture.binding_store.path().exists());
}

#[test]
fn unknown_process_identity_never_recovers_or_acknowledges() {
    let fixture = RecoveryFixture::new(ALL_EVENTS_COUNT);
    let _worker = crate::process::install_test_process_identity_relation(
        WORKER_PID,
        WORKER_START,
        crate::process::ProcessIdentityRelation::Gone,
    );
    let _process = crate::process::install_test_process_identity_relation(
        PROCESS_PID,
        PROCESS_START,
        crate::process::ProcessIdentityRelation::Unknown,
    );
    assert!(fixture.load().unwrap().is_none());
    assert!(!fixture.binding_store.path().exists());
}

fn advance(fixture: &RecoveryFixture, point: CrashPoint) {
    if matches!(point, CrashPoint::AfterEnqueue) {
        return;
    }
    let events = fixture.recovery_events();
    let (authority, _) = fixture
        .base
        .store
        .apply_recovery_in_namespace(
            &fixture.base.lease,
            fixture.authority.authentication(),
            &fixture.authority,
            &events,
        )
        .unwrap();
    if matches!(point, CrashPoint::AfterAuthorityApply) {
        return;
    }
    let pending = fixture
        .base
        .store
        .begin_binding_publication_in_namespace(
            &fixture.base.lease,
            authority.authentication(),
            &authority,
        )
        .unwrap();
    let binding = pending.binding(None).unwrap();
    fixture
        .base
        .lease
        .write_json(fixture.binding_store.path(), &binding)
        .unwrap();
    if matches!(point, CrashPoint::AfterBindingWrite) {
        return;
    }
    fixture
        .base
        .store
        .commit_binding_publication_in_namespace(
            &fixture.base.lease,
            pending.authentication(),
            &pending,
        )
        .unwrap();
    if matches!(point, CrashPoint::AfterFinalMarker) {
        return;
    }
    let pending = fixture.pending();
    crate::claude_hooks::outbox::acknowledge_recovery(
        &fixture.outbox,
        fixture.credentials,
        &pending,
    )
    .unwrap();
}

use super::super::GenerationEvent;
use super::support::{PROCESS_PID, PROCESS_START, WORKER_PID, WORKER_START};
