use super::fixture::Fixture;
use super::process_signal_child_flow_test_fixtures::{SAFE_BRIDGE, SETTLEMENT_PATH, remote_source};
use super::process_signal_test_support::messages;

const BRIDGE_PATH: &str = "core/crates/application/host/cli/src/bridge_child.rs";
const FLOW_POLICY: &str = "direct child termination is not fenced";
const REAL_RETURN: &str = "settle_failed_child(child, error); return;";

#[test]
fn macro_tokens_cannot_prove_a_bridge_return() {
    for spoof in [
        "settle_failed_child(child, error); stringify!(return);",
        "settle_failed_child(child, error); stringify![return];",
        "settle_failed_child(child, error); stringify! { return };",
    ] {
        let fixture = Fixture::new();
        fixture.write(BRIDGE_PATH, SAFE_BRIDGE.replace(REAL_RETURN, spoof));
        assert!(contains(&messages(&fixture), FLOW_POLICY));
    }
}

#[test]
fn macro_tokens_cannot_prove_a_remote_custody_transfer() {
    const REAL_RETAIN: &str = "reservation.retain(child, termination_required);";
    for spoof in [
        "stringify!(reservation.retain(child, termination_required));",
        "stringify![reservation.retain(child, termination_required)];",
        "stringify! { reservation.retain(child, termination_required) };",
    ] {
        let fixture = Fixture::new();
        fixture.write(
            SETTLEMENT_PATH,
            remote_source(SETTLEMENT_PATH).replace(REAL_RETAIN, spoof),
        );
        assert!(contains(&messages(&fixture), FLOW_POLICY));
    }
}

#[test]
fn macro_tokens_cannot_spoof_the_handoff_call() {
    let fixture = Fixture::new();
    fixture.write(
        BRIDGE_PATH,
        SAFE_BRIDGE.replace(
            REAL_RETURN,
            "stringify!(settle_failed_child(child, error)); return;",
        ),
    );
    assert!(contains(&messages(&fixture), FLOW_POLICY));
}

fn contains(messages: &[String], expected: &str) -> bool {
    messages.iter().any(|message| message.contains(expected))
}
