use super::support::reject_mutations;
use yolo_architecture_policy_contracts::process_signal as contract;

#[test]
fn broker_compensation_and_failed_handoff_preserve_exact_child_custody() {
    reject_mutations(
        contract::BROKER_PENDING_PATH,
        &[
            (
                "settle_child(detached.child, detached.termination)",
                "settle_child(detached.child, unrelated)",
            ),
            (
                "settle_child_on_drop(detached.child, detached.termination)",
                "settle_child_on_drop(unrelated, detached.termination)",
            ),
            (
                "handoff_detached_child_custody(detached.child)",
                "discard_detached_child(detached.child)",
            ),
            (
                "let child = DetachedChildCustody::capture(child)",
                "let child = DetachedChildCustody::untrusted(child)",
            ),
            (
                "let cleanup = self.compensate();",
                "let cleanup = self.discard();",
            ),
            (
                "self.detached = Some(DetachedBroker { child, termination });",
                "self.detached = None;",
            ),
        ],
    );
}

#[test]
fn explicit_and_drop_routes_cannot_discard_uncertain_children_or_errors() {
    reject_mutations(
        contract::BROKER_SETTLEMENT_PATH,
        &[
            (
                "Settlement::Settled(result) => result,",
                "Settlement::Settled(result) => Ok(()),",
            ),
            (
                "retain_unsettled(child, reservation, failure, termination_required)",
                "retain_unsettled(child, reservation, None, termination_required)",
            ),
            (
                "retain_for_drop(child, reservation, termination_required);",
                "retain_for_drop(child, unrelated, termination_required);",
            ),
            (
                "reservation.retain(child, termination_required);",
                "reservation.discard(child, termination_required);",
            ),
            (
                "reservation.retain(child, termination_required);",
                "reservation.retain(unrelated, termination_required);",
            ),
            (
                "reservation.retain(child, termination_required);",
                "reservation.retain(child, termination_required); reservation.retain(child, termination_required);",
            ),
        ],
    );
}

#[test]
fn settlement_actions_preserve_poll_order_lost_authority_and_retry_intent() {
    reject_mutations(
        contract::BROKER_SETTLEMENT_PATH,
        &[
            ("match child.poll_exited()", "match child.untrusted_poll()"),
            (
                "let termination_required = match child.terminate()",
                "let termination_required = match child.abandon()",
            ),
            (
                "let mut failures = Vec::new();",
                "let mut failures = Vec::new(); child.reap();",
            ),
            (
                "Ok(true) => return Settlement::Settled",
                "Ok(true) => return Settlement::Unrelated",
            ),
            ("if failure.wait_authority_lost()", "if failure.unrelated()"),
            ("Ok(()) => false,", "Ok(()) => true,"),
            (
                "process_is_gone(&error) => false,",
                "process_is_gone(&error) => true,",
            ),
            (
                "OperationFailure::operation(TERMINATE_CHILD_CONTEXT, error)",
                "OperationFailure::operation(TERMINATE_CHILD_CONTEXT, unrelated)",
            ),
            (
                "failures.push(OperationFailure::operation(TERMINATE_CHILD_CONTEXT, error));\n            true",
                "failures.push(OperationFailure::operation(TERMINATE_CHILD_CONTEXT, error));\n            false",
            ),
            (
                "let mut failures = Vec::new();",
                "#[cfg(not(test))] return Settlement::Settled(Ok(())); let mut failures = Vec::new();",
            ),
        ],
    );
}
