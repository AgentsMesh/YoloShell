use super::intake_task_test_support::*;
use crate::worker::codex_hook::blocking_custody::BlockingSettlementCustody;
use crate::worker::codex_hook::cancellation::IngressCancellation;
use crate::worker::codex_hook::constants::{
    CODEX_INTAKE_ENUMERATION_ERROR, CODEX_INTAKE_TASK_ERROR, CODEX_REQUEST_TIMEOUT,
};
use crate::worker::codex_hook::router::CodexRouteRetirement;
use std::sync::Arc;

#[tokio::test]
async fn cancelling_a_round_pause_releases_its_waiter_and_registration() {
    let fixture = IntakeTaskFixture::new();
    let router = fixture.router();
    let mut pausing = Box::pin(router.pause_next_intake_round_for_test());
    assert!(futures::poll!(&mut pausing).is_pending());
    drop(pausing);
    let next = router.gate_next_intake_round_for_test();
    next.release();
    super::super::intake_task::reconcile_round_for_test(router.as_ref()).unwrap();
    tokio::task::yield_now().await;
}

#[test]
fn a_route_retired_before_a_round_is_excluded_without_blocking_healthy_work() {
    let fixture = IntakeTaskFixture::new();
    let retired = fixture.route(FIRST_INVOCATION);
    let healthy = fixture.route(SECOND_INVOCATION);
    retired.persist_start();
    healthy.persist_start();
    assert_eq!(fixture.retire(&retired), CodexRouteRetirement::Retired);

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.routes, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.applied, EXPECTED_SINGLE_COUNT);
    assert_eq!(retired.pending(), EXPECTED_SINGLE_COUNT);
    assert_eq!(healthy.pending(), usize::default());
}

#[test]
fn retirement_after_enumeration_defers_only_that_route() {
    let fixture = IntakeTaskFixture::new();
    let retired = fixture.route(FIRST_INVOCATION);
    let healthy = fixture.route(SECOND_INVOCATION);
    retired.persist_start();
    healthy.persist_start();
    let router = fixture.router();
    let gate = router.gate_next_intake_round_for_test();
    let round_router = Arc::clone(&router);
    let round = std::thread::spawn(move || {
        super::super::intake_task::reconcile_round_for_test(round_router.as_ref())
    });
    gate.wait_until_entered();

    assert_eq!(fixture.retire(&retired), CodexRouteRetirement::Retired);
    gate.release();
    let report = round.join().unwrap().unwrap();

    assert_eq!(report.routes, EXPECTED_ROUTE_COUNT);
    assert_eq!(report.deferred, usize::default());
    assert_eq!(report.retirement_missing, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.applied, EXPECTED_SINGLE_COUNT);
    assert_eq!(retired.pending(), EXPECTED_SINGLE_COUNT);
    assert_eq!(healthy.pending(), usize::default());
}

#[tokio::test]
async fn cancellation_wins_before_a_poisoned_router_can_be_enumerated() {
    let fixture = IntakeTaskFixture::new();
    let router = fixture.router();
    router.poison_lock_for_test();
    let cancellation = IngressCancellation::default();
    cancellation.cancel();

    super::super::intake_task::run(router, cancellation, BlockingSettlementCustody::default())
        .await
        .unwrap();
}

#[tokio::test]
async fn router_lock_failure_stops_the_pump_fail_closed() {
    let fixture = IntakeTaskFixture::new();
    let router = fixture.router();
    router.poison_lock_for_test();

    let error = run_until_failure(router).await;

    assert!(error.contains(CODEX_INTAKE_ENUMERATION_ERROR));
    assert!(error.contains(CODEX_INTAKE_TASK_ERROR));
}

#[tokio::test]
async fn committed_authority_enumeration_failure_stops_the_pump() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.remove_admission();

    let error = run_until_failure(fixture.router()).await;

    assert!(error.contains(CODEX_INTAKE_ENUMERATION_ERROR));
    assert!(error.contains(CODEX_INTAKE_TASK_ERROR));
}

#[tokio::test]
async fn blocking_reconciliation_panic_is_contained_and_stops_the_pump() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.persist_start();
    route.runtime.panic_next_ingest_for_test();

    let error = run_until_failure(fixture.router()).await;

    assert!(error.contains(CODEX_INTAKE_TASK_ERROR));
}

async fn run_until_failure(
    router: Arc<crate::worker::codex_hook::router::CodexSessionRouter>,
) -> String {
    let result = tokio::time::timeout(
        CODEX_REQUEST_TIMEOUT,
        super::super::intake_task::run(
            router,
            IngressCancellation::default(),
            BlockingSettlementCustody::default(),
        ),
    )
    .await
    .unwrap();
    format!("{:#}", result.unwrap_err())
}
