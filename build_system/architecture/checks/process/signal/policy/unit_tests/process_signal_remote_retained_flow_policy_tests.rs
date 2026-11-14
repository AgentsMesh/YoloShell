use super::fixture::Fixture;
use super::process_signal_child_flow_test_fixtures::{
    REAPER_CUSTODY_PATH, REAPER_WORKER_PATH, SETTLEMENT_PATH, SPAWNED_PATH, remote_source,
};
use super::process_signal_test_support::messages;

const FLOW_POLICY: &str = "direct child termination is not fenced";

#[test]
fn explicit_and_drop_settlement_both_transfer_live_custody() {
    for changed in [
        remote_source(SETTLEMENT_PATH).replace(
            "retain_unsettled(child, reservation, failure, termination_required)",
            "drop(child)",
        ),
        remote_source(SETTLEMENT_PATH).replace(
            "retain_for_drop(child, reservation, termination_required);",
            "drop(child);",
        ),
    ] {
        assert_rejected(SETTLEMENT_PATH, changed);
    }
}

#[test]
fn lost_wait_authority_must_return_uncertainty() {
    let changed = remote_source(SETTLEMENT_PATH).replace(
        "return Settlement::Settled(Err(ChildSettlementFailure::one(failure).into()));",
        "record(failure);",
    );
    assert_rejected(SETTLEMENT_PATH, changed);
}

#[test]
fn retention_queue_must_publish_before_reservation_disarm() {
    let source = remote_source(REAPER_CUSTODY_PATH);
    let push = "queue.children.push_back(super::RetainedChild {\n            child,\n            termination_required,\n        });";
    let disarm = "self.active = false;";
    let changed = source
        .replace(push, "")
        .replace(disarm, &format!("{disarm}\n        {push}"));
    assert_rejected(REAPER_CUSTODY_PATH, changed);
}

#[test]
fn worker_panic_must_enter_the_retained_fallback() {
    let changed =
        remote_source(REAPER_WORKER_PATH).replace("fallback(&shared, &mut retained);", "return;");
    assert_rejected(REAPER_WORKER_PATH, changed);
}

#[test]
fn worker_removes_only_reaped_children() {
    let source = remote_source(REAPER_WORKER_PATH);
    let changed = source.replacen("children.swap_remove(index);", "record(index);", 1);
    assert_rejected(REAPER_WORKER_PATH, changed);
}

#[test]
fn failed_worker_termination_keeps_the_retry_policy() {
    let source = remote_source(REAPER_WORKER_PATH);
    let retained = "Err(error) => containment::report(error, process_id, TERMINATION_FAILURE),";
    let discarded = "Err(error) => {\n                    child.termination_required = false;\n                    containment::report(error, process_id, TERMINATION_FAILURE);\n                }";
    let changed = source.replace(retained, discarded);
    assert_ne!(changed, source);
    assert_rejected(REAPER_WORKER_PATH, changed);
}

#[test]
fn failed_generic_handoff_reconstructs_broker_custody() {
    let changed = remote_source(SPAWNED_PATH).replace(
        "self.detached = Some(DetachedBroker { child, termination });",
        "drop((child, termination));",
    );
    assert_rejected(SPAWNED_PATH, changed);
}

#[test]
fn production_cfg_cannot_bypass_settlement_actions() {
    let changed = remote_source(SETTLEMENT_PATH).replace(
        "let mut failures = Vec::new();",
        "#[cfg(not(test))] return Settlement::Settled(Ok(()));\n    let mut failures = Vec::new();",
    );
    assert_rejected(SETTLEMENT_PATH, changed);
}

fn assert_rejected(path: &str, source: String) {
    let fixture = Fixture::new();
    fixture.write(path, source);
    let result = messages(&fixture);
    assert!(
        result.iter().any(|message| message.contains(FLOW_POLICY)),
        "{result:#?}",
    );
}
