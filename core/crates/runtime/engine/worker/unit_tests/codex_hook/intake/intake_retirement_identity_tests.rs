use super::intake_retirement_test_support::*;
use super::intake_task_test_support::*;
use super::runtime_retirement_intake_test_support::replace_same_contents;
use crate::process::ProcessIdentityObservation;
use crate::worker::codex_hook::intake_task::RetirementFaultStage;

const ADMISSION_REPLACEMENT: &str = "coordinator-admission-replacement";
const OUTBOX_REPLACEMENT: &str = "coordinator-outbox-replacement";
const SEAL_REPLACEMENT: &str = "coordinator-seal-replacement";
const SNAPSHOT_REPLACEMENT: &str = "coordinator-snapshot-replacement";

#[test]
fn admission_same_payload_replacement_blocks_retirement() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    let seal_path = route
        .entry
        .admission()
        .authority()
        .retirement_seal_path_for_test();
    replace_same_contents(route.entry.admission().path(), ADMISSION_REPLACEMENT);
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    assert_reconcile_rejected(&fixture, &route);
    assert!(!seal_path.exists());
}

#[test]
fn seal_same_payload_replacement_blocks_retirement_publication() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    let path = route
        .entry
        .admission()
        .authority()
        .retirement_seal_path_for_test();
    route.runtime.hook_next_retirement_at_for_test(
        RetirementFaultStage::AfterSealPublication,
        move || {
            replace_same_contents(&path, SEAL_REPLACEMENT);
            Ok(())
        },
    );
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    assert_reconcile_rejected(&fixture, &route);
    assert!(matches!(
        retirement_state(&route),
        crate::codex_hooks::CodexRetirementState::Sealed(_)
    ));
}

#[test]
fn snapshot_same_payload_replacement_blocks_retirement_publication() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    let path = route.runtime.snapshot_path();
    replace_after_terminal_settlement(&route, path, SNAPSHOT_REPLACEMENT);
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    assert_reconcile_rejected(&fixture, &route);
    assert_sealed(&route);
}

#[test]
fn outbox_same_payload_replacement_blocks_retirement_publication() {
    let fixture = IntakeTaskFixture::new();
    let route = fixture.route(FIRST_INVOCATION);
    route.persist_start();
    let path = route.runtime.outbox_path();
    replace_after_terminal_settlement(&route, path, OUTBOX_REPLACEMENT);
    observe_next(&fixture, &route, ProcessIdentityObservation::Gone);

    assert_reconcile_rejected(&fixture, &route);
    assert_sealed(&route);
}

fn replace_after_terminal_settlement(
    route: &IntakeRoute,
    path: std::path::PathBuf,
    replacement: &'static str,
) {
    route.runtime.hook_next_retirement_at_for_test(
        RetirementFaultStage::AfterTerminalSettlement,
        move || {
            replace_same_contents(&path, replacement);
            Ok(())
        },
    );
}

fn assert_reconcile_rejected(fixture: &IntakeTaskFixture, route: &IntakeRoute) {
    assert!(
        super::super::intake_task::reconcile_route_for_test(
            fixture.router().as_ref(),
            &route.entry,
        )
        .is_err()
    );
    assert_eq!(fixture.router().len().unwrap(), SINGLE_ROUTE);
}
