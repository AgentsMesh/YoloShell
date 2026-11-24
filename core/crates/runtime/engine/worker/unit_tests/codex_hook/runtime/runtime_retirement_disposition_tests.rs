use super::exhaustion_support::install_exhausted_active_authority;
use super::runtime_intake_test_support::RoutedFixture;
use super::runtime_retirement_intake_test_support::RetirementResources;
use super::support::{Fixture, SECOND_EVENT_SEQUENCE, UNASSIGNED_EVENT_SEQUENCE, envelope};
use crate::agent::hooks::{EventKind, HookProviderKind, Reducer, SessionEndReason};
use crate::codex_hooks::{CodexOutbox, CodexRetirementDisposition};

#[test]
fn final_hook_snapshot_retires_as_clean_final() {
    let fixture = RoutedFixture::new();
    fixture
        .submit(fixture.start(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    let end = envelope(
        fixture.identity.clone(),
        SECOND_EVENT_SEQUENCE,
        Fixture::other_event_id(),
        EventKind::SessionEnd {
            reason: SessionEndReason::Other,
        },
    );
    fixture.submit(end).unwrap();
    let resources = RetirementResources::new(&fixture);

    let prepared = fixture
        .runtime
        .prepare_retirement(resources.writer(), resources.seal(), resources.admission())
        .unwrap();

    assert_eq!(
        prepared.record.disposition_for_test(),
        CodexRetirementDisposition::CleanFinal
    );
}

#[test]
fn exhausted_snapshot_without_pending_outbox_retires_canonically() {
    let fixture = RoutedFixture::new();
    install_exhausted_active_authority(&fixture);
    let (runtime, _) = fixture.restart_runtime();
    let router = fixture.router_for(&runtime);
    let resources = RetirementResources::for_route(&router, &fixture.identity);

    let prepared = runtime
        .prepare_retirement(resources.writer(), resources.seal(), resources.admission())
        .unwrap();
    let reducer = Reducer::restore(
        prepared.snapshot.clone(),
        fixture.identity.clone(),
        HookProviderKind::Codex,
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
    )
    .unwrap();

    assert!(reducer.authority_exhausted());
    assert_eq!(
        prepared.record.disposition_for_test(),
        CodexRetirementDisposition::AuthorityExhausted
    );
    drop(prepared);
    assert!(
        CodexOutbox::for_identity(runtime.outbox_path(), &fixture.identity)
            .unwrap()
            .pending()
            .unwrap()
            .is_empty()
    );
}
