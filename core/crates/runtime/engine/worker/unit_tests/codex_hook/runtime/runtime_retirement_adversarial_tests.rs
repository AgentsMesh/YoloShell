use super::runtime_intake_test_support::RoutedFixture;
use super::runtime_retirement_intake_test_support::RetirementResources;
use super::support::{INVALID_PROCESS_START_DELTA, UNASSIGNED_EVENT_SEQUENCE};
use crate::agent::hooks::ProcessIdentity;
use crate::codex_hooks::CodexOutbox;

#[test]
fn exited_snapshot_rejects_an_uncommitted_outbox_suffix() {
    let fixture = RoutedFixture::new();
    fixture
        .runtime
        .observe_process_exit(observed(&fixture))
        .unwrap();
    let outbox =
        CodexOutbox::for_identity(fixture.runtime.outbox_path(), &fixture.identity).unwrap();
    let event = outbox
        .enqueue(fixture.prompt(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    let resources = RetirementResources::new(&fixture);

    let result = fixture.runtime.prepare_retirement(
        resources.writer(),
        resources.seal(),
        resources.admission(),
    );

    assert!(result.is_err());
    assert_eq!(
        outbox.load().unwrap().pending_for_test().front(),
        Some(&event)
    );
}

#[test]
fn revoked_snapshot_rejects_an_uncommitted_outbox_suffix() {
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
    let outbox =
        CodexOutbox::for_identity(fixture.runtime.outbox_path(), &fixture.identity).unwrap();
    let event = outbox
        .enqueue(fixture.prompt(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    let resources = RetirementResources::new(&fixture);

    let result = fixture.runtime.prepare_retirement(
        resources.writer(),
        resources.seal(),
        resources.admission(),
    );

    assert!(result.is_err());
    assert_eq!(
        outbox.load().unwrap().pending_for_test().front(),
        Some(&event)
    );
}

fn observed(fixture: &RoutedFixture) -> ProcessIdentity {
    ProcessIdentity::new(fixture.process.pid, fixture.process.start_time)
}
