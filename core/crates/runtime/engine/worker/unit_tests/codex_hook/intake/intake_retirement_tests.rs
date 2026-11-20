use super::intake_retirement_test_support::*;
use super::intake_task_test_support::*;
use crate::process::ProcessIdentityObservation;

#[test]
fn matching_route_performs_normal_intake_without_retirement() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.persist_start();
    observe_next(&fixture, &route, ProcessIdentityObservation::Matching);

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.applied, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.retired, NO_ROUTES);
    assert_eq!(fixture.router().len().unwrap(), SINGLE_ROUTE);
    assert_unsealed(&route);
}

#[test]
fn unknown_route_performs_normal_intake_without_retirement() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.persist_start();
    observe_next(&fixture, &route, ProcessIdentityObservation::Unknown);

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.applied, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.retirement_pending, NO_ROUTES);
    assert_eq!(fixture.router().len().unwrap(), SINGLE_ROUTE);
    assert_unsealed(&route);
}

#[test]
fn exact_gone_route_quarantines_raw_only_intake_before_retirement() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.persist_raw_start();
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.applied, NO_ROUTES);
    assert_eq!(report.quarantined, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.retired, EXPECTED_SINGLE_COUNT);
    assert_eq!(route.pending(), NO_ROUTES);
    assert_eq!(fixture.router().len().unwrap(), NO_ROUTES);
    assert_eq!(
        route.runtime.state().unwrap().unwrap().process,
        crate::agent::hooks::ProcessState::Exited
    );
    assert_retired(&route);
}

#[test]
fn outstanding_route_lease_defers_retirement_until_release() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    let lease = fixture.router().lease_committed(&route.identity).unwrap();
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    let pending = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(pending.retirement_pending, EXPECTED_SINGLE_COUNT);
    assert_eq!(fixture.router().len().unwrap(), SINGLE_ROUTE);
    assert_unsealed(&route);
    drop(lease);
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    let retired = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(retired.retired, EXPECTED_SINGLE_COUNT);
    assert_eq!(fixture.router().len().unwrap(), NO_ROUTES);
    assert_retired(&route);
}
