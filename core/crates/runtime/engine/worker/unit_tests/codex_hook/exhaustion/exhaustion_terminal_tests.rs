use super::exhaustion_support::*;
use super::exhaustion_tests::final_event;
use super::support::*;
use crate::agent::hooks::{ProcessExitDisposition, ProcessState, Reducer};

#[test]
fn final_active_revision_is_durable_but_never_acknowledged_or_published() {
    let fixture = Fixture::new();
    install_penultimate_active_authority(&fixture);
    let (runtime, _) = fixture.restart_runtime();
    let authority = runtime.subscribe_authority();
    let outbox =
        crate::codex_hooks::CodexOutbox::for_identity(runtime.outbox_path(), &fixture.identity)
            .unwrap();
    let before = outbox.load().unwrap().acknowledged_sequence();
    let event = outbox
        .enqueue(final_event(&fixture, UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();

    assert!(matches!(
        runtime.ingest(event.clone(), fixture.process.pid),
        Err(super::super::error::CodexHookRejection::Violation(
            crate::agent::hooks::HookViolation::AuthorityExhausted
        ))
    ));
    assert!(matches!(
        authority.borrow().clone(),
        super::super::publication::CodexAuthorityView::Exhausted(_)
    ));
    let durable = restored_reducer(&fixture, &runtime);
    assert!(durable.authority_exhausted());
    assert_eq!(durable.state().last_sequence, event.sequence);
    let committed_outbox = outbox.load().unwrap();
    assert_eq!(committed_outbox.acknowledged_sequence(), before);
    assert_eq!(committed_outbox.pending_for_test().front(), Some(&event));
    let (restored, report) = fixture.restart_runtime();
    assert_eq!(report, Default::default());
    assert!(matches!(
        restored.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Exhausted(_)
    ));
    assert!(matches!(
        restored.ingest(event, fixture.process.pid),
        Err(super::super::error::CodexHookRejection::Violation(
            crate::agent::hooks::HookViolation::AuthorityExhausted
        ))
    ));
    assert!(restored.reconcile_pending().is_err());
    let observed =
        crate::agent::hooks::ProcessIdentity::new(fixture.process.pid, fixture.process.start_time);
    assert!(matches!(
        restored.observe_process_exit(observed),
        Err(super::super::error::CodexHookRejection::Violation(
            crate::agent::hooks::HookViolation::AuthorityExhausted
        ))
    ));
    assert_eq!(outbox.load().unwrap(), committed_outbox);
}

#[test]
fn exited_authority_at_last_revision_restores() {
    let fixture = Fixture::new();
    install_penultimate_active_authority(&fixture);
    let (runtime, _) = fixture.restart_runtime();
    let observed =
        crate::agent::hooks::ProcessIdentity::new(fixture.process.pid, fixture.process.start_time);

    let publication = runtime.observe_process_exit(observed).unwrap();
    let committed = runtime.state().unwrap().unwrap();
    let durable = restored_reducer(&fixture, &runtime);
    assert_eq!(publication.disposition, ProcessExitDisposition::Unconfirmed);
    assert_eq!(durable.state().process, ProcessState::Exited);
    assert_eq!(
        durable.revision(),
        Reducer::last_usable_authority_revision_for_test()
    );
    assert!(!durable.authority_exhausted());
    let (restored, report) = fixture.restart_runtime();
    assert_eq!(report, Default::default());
    assert_eq!(restored.state().unwrap(), Some(committed));
}

#[test]
fn canonical_revoked_authority_at_last_revision_restores() {
    let fixture = Fixture::new();
    install_penultimate_active_authority(&fixture);
    let (runtime, _) = fixture.restart_runtime();
    let mismatched = crate::agent::hooks::ProcessIdentity::new(
        fixture.process.pid,
        fixture
            .process
            .start_time
            .checked_add(INVALID_PROCESS_START_DELTA)
            .unwrap(),
    );

    assert!(matches!(
        runtime.observe_process_exit(mismatched),
        Err(super::super::error::CodexHookRejection::Violation(
            crate::agent::hooks::HookViolation::ProcessMismatch
        ))
    ));
    let durable = restored_reducer(&fixture, &runtime);
    assert!(durable.state().revoked);
    assert_eq!(
        durable.revision(),
        Reducer::last_usable_authority_revision_for_test()
    );
    assert!(!durable.authority_exhausted());
    let (restored, report) = fixture.restart_runtime();
    assert_eq!(report, Default::default());
    assert!(restored.state().unwrap().unwrap().revoked);
}
