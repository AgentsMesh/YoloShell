use super::process_signal_child_flow as flow;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};

#[path = "process_signal_child_flow_remote_settlement_actions.rs"]
mod actions;

const COMPENSATE: &str = "compensate";
const HANDOFF: &str = "handoff";
const DROP: &str = "drop";
const SETTLEMENT: &str = "Settlement";
const SETTLED: &str = "Settled";
const FAILURE: &str = "failure";
const RESERVATION: &str = "reservation";
const CHILD: &str = "child";
const DETACHED: &str = "detached";
const TERMINATION: &str = "termination";
const CAPTURE: &str = "capture";
const SELF_RECEIVER: &str = "self";
const MEMBER: &str = ".";
const PATH: &str = "::";
const OPEN: &str = "(";
const CLOSE: &str = ")";
const COMMA: &str = ",";
const ARROW_ASSIGN: &str = "=";
const ARROW_END: &str = ">";

pub fn spawned_flow(source: &RustSource) -> bool {
    let Some(compensate) = flow::unique_function(source, Some("PendingBroker"), COMPENSATE) else {
        return false;
    };
    let Some(drop) = flow::unique_function(source, Some("PendingBroker"), DROP) else {
        return false;
    };
    let Some(handoff) = flow::unique_function(source, Some("PendingBroker"), HANDOFF) else {
        return false;
    };
    let Some(failure) = flow::unique_function(
        source,
        Some("PendingBroker"),
        contract::COMPENSATE_HANDOFF_FAILURE,
    ) else {
        return false;
    };
    exact_detached_settlement(source, compensate, contract::SETTLE_CHILD)
        && exact_detached_settlement(source, drop, contract::SETTLE_CHILD_ON_DROP)
        && flow::ordered_once(
            source,
            handoff,
            contract::HANDOFF_DETACHED_CHILD_CUSTODY,
            contract::COMPENSATE_HANDOFF_FAILURE,
        )
        && flow::ordered_once(source, failure, CAPTURE, COMPENSATE)
        && flow::contains(
            source,
            failure,
            &[
                SELF_RECEIVER,
                MEMBER,
                DETACHED,
                ARROW_ASSIGN,
                "Some",
                OPEN,
                "DetachedBroker",
            ],
        )
}

pub fn settlement_flow(source: &RustSource) -> bool {
    let Some(settle) = flow::unique_function(source, None, contract::SETTLE_CHILD) else {
        return false;
    };
    let Some(drop) = flow::unique_function(source, None, contract::SETTLE_CHILD_ON_DROP) else {
        return false;
    };
    let Some(actions) = flow::unique_function(source, None, contract::SETTLEMENT_ACTIONS) else {
        return false;
    };
    let Some(retain) = flow::unique_function(source, None, contract::RETAIN_UNSETTLED) else {
        return false;
    };
    let Some(retain_drop) = flow::unique_function(source, None, contract::RETAIN_FOR_DROP) else {
        return false;
    };
    !flow::production_cfg_in_body(&source.tokens, actions)
        && routes_settlement(source, settle, contract::RETAIN_UNSETTLED, true)
        && routes_settlement(source, drop, contract::RETAIN_FOR_DROP, false)
        && actions::valid(source, actions)
        && exact_retain(source, retain)
        && exact_retain(source, retain_drop)
}

fn exact_detached_settlement(source: &RustSource, function: &RustFunction, name: &str) -> bool {
    flow::one_call(source, function, name).is_some()
        && flow::contains(
            source,
            function,
            &[
                name,
                OPEN,
                DETACHED,
                MEMBER,
                CHILD,
                COMMA,
                DETACHED,
                MEMBER,
                TERMINATION,
                CLOSE,
            ],
        )
}

fn routes_settlement(
    source: &RustSource,
    function: &RustFunction,
    retain: &str,
    includes_failure: bool,
) -> bool {
    let routing = if includes_failure {
        &[retain, OPEN, CHILD, COMMA, RESERVATION, COMMA, FAILURE][..]
    } else {
        &[
            retain,
            OPEN,
            CHILD,
            COMMA,
            RESERVATION,
            COMMA,
            contract::TERMINATION_REQUIRED,
        ][..]
    };
    flow::ordered_once(source, function, contract::SETTLEMENT_ACTIONS, retain)
        && (!includes_failure
            || flow::contains(
                source,
                function,
                &[
                    SETTLEMENT,
                    PATH,
                    SETTLED,
                    OPEN,
                    "result",
                    CLOSE,
                    ARROW_ASSIGN,
                    ARROW_END,
                    "result",
                ],
            ))
        && flow::contains(source, function, routing)
}

fn exact_retain(source: &RustSource, function: &RustFunction) -> bool {
    flow::one_call(source, function, contract::RETAIN).is_some()
        && flow::contains(
            source,
            function,
            &[
                RESERVATION,
                MEMBER,
                contract::RETAIN,
                OPEN,
                CHILD,
                COMMA,
                contract::TERMINATION_REQUIRED,
                CLOSE,
            ],
        )
}
