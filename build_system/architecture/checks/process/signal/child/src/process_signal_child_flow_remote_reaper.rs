use super::process_signal_child_flow as flow;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};

#[path = "process_signal_child_flow_remote_reaper_advance.rs"]
mod advance;

const RESERVATION_OWNER: &str = "TerminationReservation";
const CUSTODY_OWNER: &str = "DetachedChildCustody";
const DROP: &str = "drop";
const RECEIVE: &str = "receive";
const SLEEP: &str = "sleep";
const LOOP: &str = "loop";
const CHILDREN: &str = "children";
const CHILD: &str = "child";
const QUEUE: &str = "queue";
const RETAINED: &str = "retained";
const ACTIVE: &str = "active";
const FALSE: &str = "false";
const TRUE: &str = "true";
const MAP: &str = "map";
const EXTEND: &str = "extend";
const DRAIN: &str = "drain";
const SELF_RECEIVER: &str = "self";
const MEMBER: &str = ".";
const OPEN: &str = "(";
const CLOSE: &str = ")";
const BLOCK_OPEN: &str = "{";
const ASSIGN: &str = "=";
const PLUS: &str = "+";
const MINUS: &str = "-";
const COMMA: &str = ",";
const CONTAINMENT: &str = "containment";
const PATH_SEPARATOR: &str = "::";

pub fn custody_flow(source: &RustSource) -> bool {
    let Some(retain) = flow::unique_function(source, Some(RESERVATION_OWNER), contract::RETAIN)
    else {
        return false;
    };
    let Some(drop) = flow::unique_function(source, Some(RESERVATION_OWNER), DROP) else {
        return false;
    };
    let Some(push) = flow::one_call(source, retain, contract::PUSH_BACK) else {
        return false;
    };
    let Some(notify) = flow::one_call(source, retain, contract::NOTIFY_ONE) else {
        return false;
    };
    push < notify
        && flow::contains(
            source,
            retain,
            &[
                "if",
                contract::TERMINATION_REQUIRED,
                BLOCK_OPEN,
                QUEUE,
                MEMBER,
                "termination_count",
                PLUS,
                ASSIGN,
            ],
        )
        && flow::contains(
            source,
            retain,
            &[QUEUE, MEMBER, CHILDREN, MEMBER, contract::PUSH_BACK, OPEN],
        )
        && flow::contains(
            source,
            retain,
            &[
                "RetainedChild",
                BLOCK_OPEN,
                CHILD,
                COMMA,
                contract::TERMINATION_REQUIRED,
            ],
        )
        && flow::sequence_before(
            source,
            retain,
            &[contract::PUSH_BACK],
            &[SELF_RECEIVER, MEMBER, ACTIVE, ASSIGN, FALSE],
        )
        && flow::contains(
            source,
            drop,
            &["if", SELF_RECEIVER, MEMBER, ACTIVE, BLOCK_OPEN],
        )
        && flow::contains(source, drop, &["custody_count", MINUS, ASSIGN])
}

pub fn worker_flow(source: &RustSource) -> bool {
    let Some(run) = flow::unique_function(source, None, contract::RUN) else {
        return false;
    };
    let Some(fallback) = flow::unique_function(source, None, contract::FALLBACK) else {
        return false;
    };
    let Some(iteration) = flow::unique_function(source, None, contract::RUN_ITERATION) else {
        return false;
    };
    let Some(receive) = flow::unique_function(source, None, RECEIVE) else {
        return false;
    };
    let Some(advance) = flow::unique_function(source, None, contract::ADVANCE) else {
        return false;
    };
    let Some(adapter) = flow::unique_function(source, Some(CUSTODY_OWNER), contract::TRY_REAP)
    else {
        return false;
    };
    panic_fallback(source, run, fallback)
        && flow::ordered_once(source, iteration, RECEIVE, contract::ADVANCE)
        && receive_retains_custody(source, receive)
        && advance::retains_until_reaped(source, advance)
        && flow::contains(
            source,
            adapter,
            &[
                SELF_RECEIVER,
                MEMBER,
                contract::TRY_WAIT,
                OPEN,
                CLOSE,
                MEMBER,
                MAP,
            ],
        )
        && flow::contains(
            source,
            adapter,
            &[MEMBER, contract::IS_SOME_METHOD, OPEN, CLOSE],
        )
}

fn panic_fallback(source: &RustSource, run: &RustFunction, fallback: &RustFunction) -> bool {
    exact_containment_catch(source, run)
        && flow::ordered_once(source, run, contract::RUN_ITERATION, contract::FALLBACK)
        && flow::sequence_before(
            source,
            run,
            &[QUEUE, MEMBER, contract::WORKER_FAILED, ASSIGN, TRUE],
            &[contract::FALLBACK, OPEN],
        )
        && flow::contains(source, fallback, &[LOOP, BLOCK_OPEN])
        && exact_containment_catch(source, fallback)
        && flow::ordered_once(source, fallback, contract::RUN_ITERATION, SLEEP)
}

fn exact_containment_catch(source: &RustSource, function: &RustFunction) -> bool {
    matches!(flow::calls(&source.tokens, function, contract::CATCH_UNWIND).as_slice(),
        [call] if source.tokens.get(call.saturating_sub(2)).is_some_and(|token| token.identifier(CONTAINMENT))
            && source.tokens.get(call.saturating_sub(1)).is_some_and(|token| token.symbol(PATH_SEPARATOR)))
}

fn receive_retains_custody(source: &RustSource, function: &RustFunction) -> bool {
    flow::contains(
        source,
        function,
        &[
            RETAINED, MEMBER, EXTEND, OPEN, QUEUE, MEMBER, CHILDREN, MEMBER, DRAIN, OPEN,
        ],
    )
}
