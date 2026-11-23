use super::exhaustion_support::*;
use super::support::*;

const FINAL_EVENT_UUID: u128 = 0x82000000000000000000000000000000;

#[test]
fn exhausted_active_snapshot_restores_without_live_process_admission() {
    let fixture = Fixture::new();
    install_exhausted_active_authority(&fixture);

    let (runtime, report) = super::super::runtime::CodexHookRuntime::new_test(
        &fixture.paths,
        Fixture::terminal_id(),
        Fixture::runtime_epoch(),
        fixture.shell,
        fixture.identity.clone(),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        super::process_support::failing_process_source(),
    )
    .unwrap();

    assert_eq!(report, Default::default());
    assert!(runtime.state().unwrap().is_some());
    assert!(matches!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Exhausted(_)
    ));
    assert!(!runtime.storage_failed().unwrap());
}

#[test]
fn startup_pending_final_revision_recovers_as_exhausted() {
    let fixture = Fixture::new();
    install_penultimate_active_authority(&fixture);
    let outbox = crate::codex_hooks::CodexOutbox::for_identity(
        fixture.runtime.outbox_path(),
        &fixture.identity,
    )
    .unwrap();
    let event = outbox
        .enqueue(final_event(&fixture, UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();

    let (runtime, report) = fixture.restart_runtime();

    assert_eq!(report, Default::default());
    assert_eq!(
        runtime.state().unwrap().unwrap().last_sequence,
        event.sequence
    );
    assert!(matches!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Exhausted(_)
    ));
    assert_eq!(
        outbox.load().unwrap().pending_for_test().front(),
        Some(&event)
    );
}

#[test]
fn exhausted_snapshot_with_retained_suffix_restores_repeatedly() {
    let fixture = Fixture::new();
    install_penultimate_active_authority(&fixture);
    let outbox = crate::codex_hooks::CodexOutbox::for_identity(
        fixture.runtime.outbox_path(),
        &fixture.identity,
    )
    .unwrap();
    let final_event = outbox
        .enqueue(final_event(&fixture, UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    let suffix = outbox
        .enqueue(fixture.prompt(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    let outbox_before = std::fs::read(fixture.runtime.outbox_path()).unwrap();

    let (first, first_report) = fixture.restart_runtime();

    assert_eq!(first_report, Default::default());
    assert_restored_exhausted(&first, final_event.sequence);
    drop(first);

    let (second, second_report) = fixture.restart_runtime();
    assert_eq!(second_report, Default::default());
    assert_restored_exhausted(&second, final_event.sequence);
    assert_eq!(
        outbox.load().unwrap().pending_for_test().back(),
        Some(&suffix)
    );
    assert_eq!(
        std::fs::read(fixture.runtime.outbox_path()).unwrap(),
        outbox_before
    );
}

fn assert_restored_exhausted(
    runtime: &super::super::runtime::CodexHookRuntime,
    expected: crate::agent::hooks::EventSequence,
) {
    assert_eq!(runtime.state().unwrap().unwrap().last_sequence, expected);
    assert!(matches!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Exhausted(_)
    ));
}

pub(super) fn final_event(fixture: &Fixture, sequence: u64) -> crate::agent::hooks::HookEnvelope {
    envelope(
        fixture.identity.clone(),
        sequence,
        uuid::Uuid::from_u128(FINAL_EVENT_UUID),
        crate::agent::hooks::EventKind::UserPromptSubmit,
    )
}
