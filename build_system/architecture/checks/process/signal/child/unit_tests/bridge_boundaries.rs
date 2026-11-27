use super::constants::REORDERED_CHILD_CALL;
use super::support::{assert_flow, reject_mutations, source};
use yolo_architecture_policy_contracts::process_signal as contract;

const ROOTED_KILL: &str = "::std::process::Child::kill";
const ROOTED_WAIT: &str = "::std::process::Child::wait";

#[test]
fn bridge_requires_real_poll_evidence_and_both_cleanup_operations() {
    reject_mutations(
        contract::BRIDGE_CHILD_PATH,
        &[
            (
                "::std::process::Child::kill",
                "::std::process::Child::ignore",
            ),
            (
                "::std::process::Child::wait",
                "::std::process::Child::ignore",
            ),
            (
                "let observed = ::std::process::Child::try_wait",
                "let other_observation = ::std::process::Child::try_wait",
            ),
            ("Ok(None) => {", "Ok(Some(value)) => {"),
            (
                "Err(error) => settle_failed_child",
                "Err(error) => record_error",
            ),
            (
                "match observed {",
                "let ignored = something.ok(); match observed {",
            ),
        ],
    );
}

#[test]
fn bridge_cannot_wait_before_termination_even_when_each_call_exists_once() {
    let baseline = source(contract::BRIDGE_CHILD_PATH);
    let reordered = baseline
        .replace(ROOTED_KILL, REORDERED_CHILD_CALL)
        .replace(ROOTED_WAIT, ROOTED_KILL)
        .replace(REORDERED_CHILD_CALL, ROOTED_WAIT);
    assert_ne!(reordered, baseline);
    assert_flow(contract::BRIDGE_CHILD_PATH, &reordered, false);
}
