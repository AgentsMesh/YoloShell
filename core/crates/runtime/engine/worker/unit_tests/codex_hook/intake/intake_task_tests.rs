use super::intake_task_test_support::*;

#[test]
fn one_round_reconciles_multiple_committed_invocations() {
    let fixture = IntakeTaskFixture::new();
    let first = fixture.route(FIRST_INVOCATION);
    let second = fixture.route(SECOND_INVOCATION);
    first.persist_start();
    second.persist_start();

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.routes, EXPECTED_ROUTE_COUNT);
    assert_eq!(report.applied, EXPECTED_ROUTE_COUNT);
    assert_eq!(report.failed_routes, usize::default());
    assert_eq!(first.pending(), usize::default());
    assert_eq!(second.pending(), usize::default());
}

#[test]
fn a_failed_early_route_does_not_block_a_later_healthy_route() {
    let fixture = IntakeTaskFixture::new();
    let failed = fixture.route(FIRST_INVOCATION);
    let healthy = fixture.route(SECOND_INVOCATION);
    failed.break_intake_storage();
    healthy.persist_start();

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.routes, EXPECTED_ROUTE_COUNT);
    assert_eq!(report.failed_routes, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.applied, EXPECTED_SINGLE_COUNT);
    assert_eq!(healthy.pending(), usize::default());
}

#[test]
fn a_raw_only_route_is_deferred_without_blocking_an_authoritative_route() {
    let fixture = IntakeTaskFixture::new();
    let raw_only = fixture.route(FIRST_INVOCATION);
    let healthy = fixture.route(SECOND_INVOCATION);
    raw_only.persist_raw_prompt();
    healthy.persist_start();

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.routes, EXPECTED_ROUTE_COUNT);
    assert_eq!(report.quarantined, usize::default());
    assert_eq!(report.deferred, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.applied, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.failed_routes, usize::default());
    assert_eq!(raw_only.pending(), EXPECTED_SINGLE_COUNT);
    let outbox = crate::codex_hooks::CodexOutbox::for_identity(
        raw_only.runtime.outbox_path(),
        &raw_only.identity,
    )
    .unwrap();
    assert!(outbox.load().unwrap().pending_for_test().is_empty());
    assert_eq!(
        raw_only.runtime.state().unwrap().unwrap().last_sequence,
        crate::agent::hooks::EventSequence::ZERO
    );
    assert_eq!(healthy.pending(), usize::default());
}
