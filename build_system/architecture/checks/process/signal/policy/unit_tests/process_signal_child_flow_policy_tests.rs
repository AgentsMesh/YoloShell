use super::fixture::Fixture;
use super::process_signal_child_flow_test_fixtures::{
    REAPER_WORKER_PATH, SAFE_BRIDGE, SETTLEMENT_PATH, remote_source, write_remote_contract,
};
use super::process_signal_test_support::messages;
use yolo_architecture_policy_contracts::process_signal as contract;

const BRIDGE_PATH: &str = "core/crates/application/host/cli/src/bridge_child.rs";
const FLOW_POLICY: &str = "direct child termination is not fenced";
const RAW_POLICY: &str = "raw process signal primitive";

#[test]
fn accepts_fail_closed_child_settlement_order() {
    let fixture = Fixture::new();
    fixture.write(BRIDGE_PATH, SAFE_BRIDGE);
    write_remote_contract(&fixture);
    let result = messages(&fixture);
    assert!(result.is_empty(), "{result:#?}");
}

#[test]
fn rejects_wait_after_bridge_termination_failure() {
    for changed in [
        SAFE_BRIDGE.replace(
            "settle_failed_child(child, error); return;",
            "settle_failed_child(child, error);",
        ),
        SAFE_BRIDGE.replace(
            "settle_failed_child(child, error); return;",
            "settle_failed_child(child, error); if unrelated() { return; }",
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(BRIDGE_PATH, changed);
        assert!(contains(&messages(&fixture), FLOW_POLICY));
    }
}

#[test]
fn rejects_poll_error_cleanup_and_shadowed_bridge_child() {
    let fixture = Fixture::new();
    fixture.write(
        BRIDGE_PATH,
        SAFE_BRIDGE.replace(
            "let observed = ::std::process::Child::try_wait(&mut child);",
            "let mut child = replacement(); let observed = ::std::process::Child::try_wait(&mut child);",
        ),
    );
    assert!(contains(&messages(&fixture), FLOW_POLICY));
}

#[test]
fn rejects_spoofed_bridge_kill_result_and_handoff() {
    for changed in [
        SAFE_BRIDGE.replace(
            "let terminated = ::std::process::Child::kill(&mut child);",
            "let terminated = prior_result; ::std::process::Child::kill(&mut child);",
        ),
        SAFE_BRIDGE.replace(
            "settle_failed_child(child, error); return;",
            "record_termination_failure(error); return;",
        ),
        SAFE_BRIDGE.replace(
            "Err(error) => settle_failed_child(child, error),",
            "Err(error) => { let _ = settle_failed_child; record(error); },",
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(BRIDGE_PATH, changed);
        assert!(contains(&messages(&fixture), FLOW_POLICY));
    }
}

#[test]
fn remote_termination_failure_must_remain_retained_for_retry() {
    let fixture = Fixture::new();
    let changed = remote_source(SETTLEMENT_PATH).replace(
        "failures.push(OperationFailure::operation(TERMINATE_CHILD_CONTEXT, error));\n            true",
        "failures.push(OperationFailure::operation(TERMINATE_CHILD_CONTEXT, error));\n            false",
    );
    fixture.write(SETTLEMENT_PATH, changed);
    assert!(contains(&messages(&fixture), FLOW_POLICY));
}

#[test]
fn remote_settlement_cannot_reap_synchronously() {
    let fixture = Fixture::new();
    let changed = remote_source(SETTLEMENT_PATH).replace(
        "let mut failures = Vec::new();",
        "let mut failures = Vec::new(); child.reap();",
    );
    fixture.write(SETTLEMENT_PATH, changed);
    assert!(contains(&messages(&fixture), FLOW_POLICY));
}

#[test]
fn remote_reaper_requires_the_exact_containment_boundary() {
    let qualified = format!("containment::{}", contract::CATCH_UNWIND);
    for changed in [
        remote_source(REAPER_WORKER_PATH).replace(&qualified, contract::CATCH_UNWIND),
        remote_source(REAPER_WORKER_PATH)
            .replace(&qualified, &format!("other::{}", contract::CATCH_UNWIND)),
    ] {
        let fixture = Fixture::new();
        fixture.write(REAPER_WORKER_PATH, changed);
        assert!(contains(&messages(&fixture), FLOW_POLICY));
    }
}

#[test]
fn try_wait_error_can_never_fall_through_to_numeric_kill() {
    let fixture = Fixture::new();
    fixture.write(
        BRIDGE_PATH,
        r#"
impl Drop for SpawnedHost {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else { return; };
        match Child::try_wait(&mut child) {
            Err(error) => { libc::kill(child.id() as i32, 9); }
            _ => {}
        }
    }
}
"#,
    );
    let result = messages(&fixture);
    assert!(contains(&result, FLOW_POLICY));
    assert!(contains(&result, RAW_POLICY));
}

fn contains(messages: &[String], expected: &str) -> bool {
    messages.iter().any(|message| message.contains(expected))
}
