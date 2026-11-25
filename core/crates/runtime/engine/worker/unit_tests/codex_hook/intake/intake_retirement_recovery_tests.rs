use super::intake_retirement_test_support::*;
use super::intake_task_test_support::*;
use super::runtime_retirement_intake_test_support::replace_same_contents;
use crate::process::ProcessIdentityObservation;
use crate::worker::codex_hook::intake_task::RetirementFaultStage;

const OUTBOX_REPLACEMENT_FAULT: &str = "injected terminal outbox replacement";
const OUTBOX_PUBLICATION_REPLACEMENT: &str = "published-outbox-replacement";

#[test]
fn seal_only_retry_completes_without_another_gone_proof() {
    let (fixture, route) = fail_at(RetirementFaultStage::AfterSealPublication);

    assert_sealed(&route);
    retry_with_unknown(&fixture, &route);
}

#[test]
fn terminal_snapshot_retry_completes_without_another_gone_proof() {
    let (fixture, route) = fail_at(RetirementFaultStage::AfterTerminalSettlement);

    assert_sealed(&route);
    assert_eq!(
        route.runtime.state().unwrap().unwrap().process,
        crate::agent::hooks::ProcessState::Exited
    );
    retry_with_unknown(&fixture, &route);
}

#[test]
fn published_record_retry_removes_exact_route_without_gone_proof() {
    let (fixture, route) = fail_at(RetirementFaultStage::AfterRetirementPublication);

    assert_retired(&route);
    assert_eq!(fixture.router().len().unwrap(), SINGLE_ROUTE);
    retry_with_unknown(&fixture, &route);
}

#[test]
fn published_record_retry_revalidates_terminal_outbox() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.persist_start();
    let outbox_path = route.runtime.outbox_path();
    route.runtime.hook_next_retirement_at_for_test(
        RetirementFaultStage::AfterRetirementPublication,
        move || {
            let replacement =
                serde_json::to_vec_pretty(&crate::codex_hooks::CodexOutboxState::default())?;
            std::fs::write(outbox_path, replacement)?;
            anyhow::bail!(OUTBOX_REPLACEMENT_FAULT)
        },
    );
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    let published = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(published.failed_routes, EXPECTED_SINGLE_COUNT);
    assert_retired(&route);
    assert_eq!(fixture.router().len().unwrap(), SINGLE_ROUTE);
    observe_next(&fixture, &route, ProcessIdentityObservation::Unknown);

    let retried = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(retried.failed_routes, EXPECTED_SINGLE_COUNT);
    assert_eq!(retried.retired, NO_ROUTES);
    assert_eq!(fixture.router().len().unwrap(), SINGLE_ROUTE);
    assert_retired(&route);
}

#[test]
fn publication_window_outbox_replacement_blocks_route_deletion() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.persist_start();
    let outbox_path = route.runtime.outbox_path();
    route.runtime.hook_next_retirement_at_for_test(
        RetirementFaultStage::AfterRetirementPublication,
        move || {
            replace_same_contents(&outbox_path, OUTBOX_PUBLICATION_REPLACEMENT);
            Ok(())
        },
    );
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.failed_routes, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.retired, NO_ROUTES);
    assert_eq!(fixture.router().len().unwrap(), SINGLE_ROUTE);
    assert_retired(&route);
}

#[test]
fn sealed_route_rejects_normal_lease_but_consumes_terminal_occurrence() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.persist_start();
    route
        .runtime
        .fail_next_retirement_at_for_test(RetirementFaultStage::AfterSealPublication);
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    let failed = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(failed.failed_routes, EXPECTED_SINGLE_COUNT);
    assert_eq!(route.pending(), EXPECTED_SINGLE_COUNT);
    assert!(fixture.router().lease_committed(&route.identity).is_err());
    observe_next(&fixture, &route, ProcessIdentityObservation::Unknown);

    let retired = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(retired.applied, EXPECTED_SINGLE_COUNT);
    assert_eq!(retired.retired, EXPECTED_SINGLE_COUNT);
    assert_eq!(route.pending(), NO_ROUTES);
    assert_retired(&route);
}

fn fail_at(stage: RetirementFaultStage) -> (IntakeTaskFixture, IntakeRoute) {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.runtime.fail_next_retirement_at_for_test(stage);
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.failed_routes, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.retired, NO_ROUTES);
    assert_eq!(fixture.router().len().unwrap(), SINGLE_ROUTE);
    (fixture, route)
}

fn retry_with_unknown(fixture: &IntakeTaskFixture, route: &IntakeRoute) {
    observe_next(fixture, route, ProcessIdentityObservation::Unknown);

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.failed_routes, NO_ROUTES);
    assert_eq!(report.retired, EXPECTED_SINGLE_COUNT);
    assert_eq!(fixture.router().len().unwrap(), NO_ROUTES);
    assert_retired(route);
}
