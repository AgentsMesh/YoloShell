use super::exhaustion_support::*;
use super::exhaustion_tests::final_event;
use super::support::*;
use crate::codex_hooks::{fail_snapshot_after_commit, fail_snapshot_before_commit};

#[test]
fn final_snapshot_failure_before_commit_preserves_penultimate_authority() {
    let fixture = Fixture::new();
    install_penultimate_active_authority(&fixture);
    let (runtime, _) = fixture.restart_runtime();
    let snapshot_before = std::fs::read(runtime.snapshot_path()).unwrap();
    let event = enqueue_final(&fixture, &runtime);
    let outbox_before = load_outbox(&fixture, &runtime);
    fail_snapshot_before_commit(&runtime.snapshot_path());

    assert_authority_lost(runtime.ingest(event.clone(), fixture.process.pid));

    assert_authority_lost_latched(&runtime);
    assert_eq!(
        std::fs::read(runtime.snapshot_path()).unwrap(),
        snapshot_before
    );
    assert_eq!(load_outbox(&fixture, &runtime), outbox_before);
    let (restored, report) = fixture.restart_runtime();
    assert_eq!(report, Default::default());
    assert_exhausted(&restored, event.sequence.0);
}

#[test]
fn uncertain_final_snapshot_commit_recovers_as_exhausted() {
    let fixture = Fixture::new();
    install_penultimate_active_authority(&fixture);
    let (runtime, _) = fixture.restart_runtime();
    let event = enqueue_final(&fixture, &runtime);
    let outbox_before = load_outbox(&fixture, &runtime);
    fail_snapshot_after_commit(&runtime.snapshot_path());

    assert_authority_lost(runtime.ingest(event.clone(), fixture.process.pid));

    assert_authority_lost_latched(&runtime);
    assert_eq!(load_outbox(&fixture, &runtime), outbox_before);
    assert!(restored_reducer(&fixture, &runtime).authority_exhausted());
    let (restored, report) = fixture.restart_runtime();
    assert_eq!(report, Default::default());
    assert_exhausted(&restored, event.sequence.0);
}

fn enqueue_final(
    fixture: &Fixture,
    runtime: &super::super::runtime::CodexHookRuntime,
) -> crate::agent::hooks::HookEnvelope {
    crate::codex_hooks::CodexOutbox::for_identity(runtime.outbox_path(), &fixture.identity)
        .unwrap()
        .enqueue(final_event(fixture, UNASSIGNED_EVENT_SEQUENCE))
        .unwrap()
}

fn load_outbox(
    fixture: &Fixture,
    runtime: &super::super::runtime::CodexHookRuntime,
) -> crate::codex_hooks::CodexOutboxState {
    crate::codex_hooks::CodexOutbox::for_identity(runtime.outbox_path(), &fixture.identity)
        .unwrap()
        .load()
        .unwrap()
}

fn assert_authority_lost(
    result: Result<
        super::super::runtime::CodexHookPublication,
        super::super::error::CodexHookRejection,
    >,
) {
    assert!(matches!(
        result,
        Err(super::super::error::CodexHookRejection::AuthorityLost(_))
    ));
}

fn assert_authority_lost_latched(runtime: &super::super::runtime::CodexHookRuntime) {
    assert_eq!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::AuthorityLost
    );
}

fn assert_exhausted(runtime: &super::super::runtime::CodexHookRuntime, sequence: u64) {
    assert_eq!(runtime.state().unwrap().unwrap().last_sequence.0, sequence);
    assert!(matches!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Exhausted(_)
    ));
}
