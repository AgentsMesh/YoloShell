use super::fixture::Fixture;
use super::process_signal_child_flow_test_fixtures::{remote_source, write_remote_contract};
use super::process_signal_test_support::messages;
use yolo_architecture_policy_contracts::process_signal::{
    REQUEST_TERMINATION, TERMINATE_AND_WAIT_STATUS,
};

const COMMAND_SETTLEMENT: &str = "core/crates/runtime/engine/process/src/command/command_settlement.rs";
const PROCESS_REAPER: &str = "core/crates/runtime/engine/process/src/child/child_reaper_worker.rs";
const CUSTODY: &str = "core/crates/runtime/engine/process/src/detached_child_custody.rs";
const SCOPED_CUSTODY: &str = "core/crates/runtime/engine/process/src/scoped/scoped_waitable_child.rs";
const FUTURE: &str = "core/crates/runtime/engine/process/src/future_cleanup.rs";
const FUTURE_TEST: &str = "core/crates/runtime/engine/process/src/future_cleanup_tests.rs";
const POLICY: &str = "direct child termination request escaped";

#[test]
fn accepts_only_the_complete_retained_custody_graph() {
    let fixture = Fixture::new();
    write_remote_contract(&fixture);
    for path in [COMMAND_SETTLEMENT, PROCESS_REAPER, CUSTODY, SCOPED_CUSTODY] {
        fixture.write(path, remote_source(path));
    }
    let result = messages(&fixture);
    assert!(result.is_empty(), "{result:#?}");
}

#[test]
fn rejects_direct_requests_from_future_business_code() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE,
        "fn cleanup(child: &mut DetachedChildCustody) { child.request_direct_termination(); }",
    );
    assert_eq!(count(&messages(&fixture)), 1);
}

#[test]
fn rejects_aliases_and_function_references() {
    for source in [
        "use crate::process::request_direct_termination as terminate;",
        "fn cleanup(child: Child) { let terminate = child.request_direct_termination; }",
        "fn cleanup(child: Child) { child.request_direct_termination(); child.request_direct_termination(); }",
    ] {
        let fixture = Fixture::new();
        fixture.write(FUTURE, source);
        assert!(count(&messages(&fixture)) > 0, "{source}");
    }
}

#[test]
fn every_required_command_settlement_call_is_present_once() {
    let source = remote_source(COMMAND_SETTLEMENT);
    for changed in [
        source.replacen("child.request_direct_termination()", "Ok(())", 1),
        source.replacen(
            "child.request_direct_termination()",
            "{ child.request_direct_termination()?; child.request_direct_termination() }",
            1,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(COMMAND_SETTLEMENT, changed);
        assert!(count(&messages(&fixture)) > 0);
    }
}

#[test]
fn scoped_custody_call_cannot_move_or_duplicate() {
    let source = remote_source(SCOPED_CUSTODY);
    for changed in [
        source.replace(TERMINATE_AND_WAIT_STATUS, REQUEST_TERMINATION),
        source.replacen(
            "if let Err(error) = custody.request_direct_termination()",
            "if let Err(error) = Ok(())",
            1,
        ),
        source.replacen(
            "if let Err(error) = custody.request_direct_termination()",
            "if let Err(error) = { custody.request_direct_termination()?; custody.request_direct_termination() }",
            1,
        ),
        source.replacen(
            "let termination = custody.request_direct_termination();",
            "let termination = Ok(());",
            1,
        ),
        source.replacen(
            "let termination = custody.request_direct_termination();",
            "let termination = { custody.request_direct_termination()?; custody.request_direct_termination() };",
            1,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(SCOPED_CUSTODY, changed);
        assert!(count(&messages(&fixture)) > 0);
    }
}

#[test]
fn comments_strings_and_test_sources_do_not_gain_production_authority() {
    let comments = Fixture::new();
    comments.write(
        FUTURE,
        r#"
// child.request_direct_termination();
const NOTE: &str = "child.request_direct_termination()";
fn cleanup() {}
"#,
    );
    assert_eq!(count(&messages(&comments)), 0);

    let tests = Fixture::new();
    tests.write(
        FUTURE_TEST,
        "fn cleanup(child: &mut Child) { child.request_direct_termination(); }",
    );
    assert_eq!(count(&messages(&tests)), 0);
}

fn count(messages: &[String]) -> usize {
    messages
        .iter()
        .filter(|message| message.contains(POLICY))
        .count()
}
