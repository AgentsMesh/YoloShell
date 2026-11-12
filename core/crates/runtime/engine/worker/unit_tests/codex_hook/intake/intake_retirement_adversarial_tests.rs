use super::exhaustion_support::install_exhausted_active_authority;
use super::intake_retirement_test_support::*;
use super::intake_task_test_support::*;
use super::runtime_intake_test_support::RoutedFixture;
use super::runtime_retirement_intake_test_support::intake;
use super::support::INVALID_PROCESS_START_DELTA;
use crate::process::ProcessIdentityObservation;
use crate::worker::codex_hook::intake_task::RetirementFaultStage;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

#[test]
fn deferred_terminal_occurrence_blocks_retirement_publication() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.persist_start();
    let gate = route.runtime.gate_next_intake_authorization_for_test();
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);
    let router = fixture.router();
    let round_router = Arc::clone(&router);
    let round = std::thread::spawn(move || {
        super::super::intake_task::reconcile_round_for_test(round_router.as_ref())
    });
    gate.wait_until_entered();

    super::runtime_retirement_intake_test_support::replace_occurrence(route.intake_path());
    gate.release();
    let report = round.join().unwrap().unwrap();

    assert_eq!(report.deferred, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.retirement_pending, EXPECTED_SINGLE_COUNT);
    assert_eq!(report.retired, NO_ROUTES);
    assert_eq!(router.len().unwrap(), SINGLE_ROUTE);
    assert_sealed(&route);
}

#[test]
fn final_intake_scan_becoming_nonempty_blocks_record_publication() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.persist_start();
    let (path, contents) = only_occurrence(route.intake_path());
    route.runtime.hook_next_retirement_at_for_test(
        RetirementFaultStage::BeforeTerminalIntakeProof,
        move || {
            std::fs::write(&path, contents)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE))?;
            Ok(())
        },
    );
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    let blocked = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(blocked.failed_routes, EXPECTED_SINGLE_COUNT);
    assert_eq!(blocked.retired, NO_ROUTES);
    assert_eq!(route.pending(), EXPECTED_SINGLE_COUNT);
    assert_eq!(fixture.router().len().unwrap(), SINGLE_ROUTE);
    assert_sealed(&route);
}

#[test]
fn exact_gone_revoked_route_retires_with_canonical_disposition() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    let mismatched = crate::agent::hooks::ProcessIdentity::new(
        route.identity.process.pid,
        route
            .identity
            .process
            .start_time
            .saturating_add(INVALID_PROCESS_START_DELTA),
    );
    assert!(route.runtime.observe_process_exit(mismatched).is_err());
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    let report = super::super::intake_task::reconcile_round_for_test(&fixture.router()).unwrap();

    assert_eq!(report.retired, EXPECTED_SINGLE_COUNT);
    assert!(route.runtime.state().unwrap().unwrap().revoked);
    assert_retired(&route);
}

#[test]
fn exact_gone_exhausted_route_retires_with_canonical_disposition() {
    let fixture = RoutedFixture::new();
    install_exhausted_active_authority(&fixture);
    let _intake = intake(&fixture);
    let (runtime, _) = fixture.restart_runtime();
    let router = fixture.router_for(&runtime);
    let entry = router.resolve(&fixture.identity).unwrap();
    router.set_next_process_observation_for_test(&entry, ProcessIdentityObservation::Gone);

    super::super::intake_task::reconcile_route_for_test(&router, &entry).unwrap();

    assert_eq!(router.len().unwrap(), NO_ROUTES);
    let admission = entry.admission().authority();
    let writer = admission.acquire_retirement_writer_fence().unwrap();
    assert!(matches!(
        writer.read_state(admission).unwrap(),
        crate::codex_hooks::CodexRetirementState::Retired(_)
    ));
}
