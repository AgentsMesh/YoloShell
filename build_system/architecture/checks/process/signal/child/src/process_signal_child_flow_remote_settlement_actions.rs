use crate::process_signal_child_flow as flow;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};

const SETTLEMENT: &str = "Settlement";
const SETTLED: &str = "Settled";
const RETAIN_VARIANT: &str = "Retain";
const OK: &str = "Ok";
const TRUE: &str = "true";
const FALSE: &str = "false";
const RETURN: &str = "return";
const FAILURE: &str = "failure";
const FAILURES: &str = "failures";
const ERROR_BINDING: &str = "error";
const TERMINATE_CONTEXT: &str = "TERMINATE_CHILD_CONTEXT";
const MEMBER: &str = ".";
const PATH: &str = "::";
const OPEN: &str = "(";
const CLOSE: &str = ")";
const BLOCK_OPEN: &str = "{";
const COMMA: &str = ",";
const ARROW_ASSIGN: &str = "=";
const ARROW_END: &str = ">";
const STATEMENT_END: &str = ";";
const REFERENCE: &str = "&";
const PUSH: &str = "push";
const OPERATION_FAILURE: &str = "OperationFailure";

pub(super) fn valid(source: &RustSource, function: &RustFunction) -> bool {
    let Some(poll) = flow::one_call(source, function, contract::POLL_EXITED) else {
        return false;
    };
    let Some(terminate) = flow::one_call(source, function, contract::TERMINATE) else {
        return false;
    };
    poll < terminate
        && flow::calls(&source.tokens, function, contract::REAP).is_empty()
        && flow::contains(
            source,
            function,
            &[
                OK,
                OPEN,
                TRUE,
                CLOSE,
                ARROW_ASSIGN,
                ARROW_END,
                RETURN,
                SETTLEMENT,
                PATH,
                SETTLED,
            ],
        )
        && flow::contains(
            source,
            function,
            &[
                contract::WAIT_AUTHORITY_LOST,
                OPEN,
                CLOSE,
                BLOCK_OPEN,
                RETURN,
                SETTLEMENT,
                PATH,
                SETTLED,
            ],
        )
        && flow::contains(
            source,
            function,
            &[OK, OPEN, OPEN, CLOSE, CLOSE, ARROW_ASSIGN, ARROW_END, FALSE],
        )
        && flow::contains(
            source,
            function,
            &[
                contract::PROCESS_IS_GONE,
                OPEN,
                REFERENCE,
                ERROR_BINDING,
                CLOSE,
                ARROW_ASSIGN,
                ARROW_END,
                FALSE,
            ],
        )
        && flow::contains(
            source,
            function,
            &[
                FAILURES,
                MEMBER,
                PUSH,
                OPEN,
                OPERATION_FAILURE,
                PATH,
                contract::OPERATION,
                OPEN,
                TERMINATE_CONTEXT,
                COMMA,
                ERROR_BINDING,
                CLOSE,
                CLOSE,
                STATEMENT_END,
                TRUE,
            ],
        )
        && flow::contains(
            source,
            function,
            &[
                SETTLEMENT,
                PATH,
                RETAIN_VARIANT,
                BLOCK_OPEN,
                FAILURE,
                COMMA,
                contract::TERMINATION_REQUIRED,
            ],
        )
}
