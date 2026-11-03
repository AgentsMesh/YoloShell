use crate::process_signal_child_flow as flow;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};

const CHILD: &str = "child";
const PROGRESS: &str = "progress";
const TERMINATIONS: &str = "terminations";
const SETTLED: &str = "settled";
const INDEX: &str = "index";
const ERROR: &str = "error";
const IF: &str = "if";
const MATCH: &str = "match";
const FALSE: &str = "false";
const MEMBER: &str = ".";
const OPEN: &str = "(";
const CLOSE: &str = ")";
const BLOCK_OPEN: &str = "{";
const ASSIGN: &str = "=";
const PLUS: &str = "+";
const REFERENCE: &str = "&";
const REQUIRED_REMOVALS: usize = 2;
const REQUIRED_ASSIGNMENTS: usize = 2;

pub(super) fn retains_until_reaped(source: &RustSource, function: &RustFunction) -> bool {
    let polls = flow::calls(&source.tokens, function, contract::TRY_REAP);
    let terms = flow::calls(&source.tokens, function, contract::TERMINATE);
    let removals = flow::calls(&source.tokens, function, contract::SWAP_REMOVE);
    matches!((polls.as_slice(), terms.as_slice()), ([poll], [term]) if poll < term)
        && removals.len() == REQUIRED_REMOVALS
        && flow::assignment_count(source, function, contract::TERMINATION_REQUIRED, FALSE)
            == REQUIRED_ASSIGNMENTS
        && flow::contains(
            source,
            function,
            &[
                contract::IS_CHILD_ALREADY_REAPED,
                OPEN,
                REFERENCE,
                ERROR,
                CLOSE,
            ],
        )
        && flow::contains(
            source,
            function,
            &[
                IF,
                CHILD,
                MEMBER,
                contract::TERMINATION_REQUIRED,
                BLOCK_OPEN,
                MATCH,
                CHILD,
                MEMBER,
                CHILD,
                MEMBER,
                contract::TERMINATE,
            ],
        )
        && flow::contains(
            source,
            function,
            &[contract::PROCESS_IS_GONE, OPEN, REFERENCE, ERROR, CLOSE],
        )
        && flow::contains(
            source,
            function,
            &[PROGRESS, MEMBER, TERMINATIONS, PLUS, ASSIGN],
        )
        && flow::contains(source, function, &[PROGRESS, MEMBER, SETTLED, PLUS, ASSIGN])
        && flow::contains(source, function, &[INDEX, PLUS, ASSIGN])
}
