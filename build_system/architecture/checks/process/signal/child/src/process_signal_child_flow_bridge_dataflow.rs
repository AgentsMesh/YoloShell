use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_token_sequence as sequence;
use yolo_architecture_syntax::rust_source::RustFunction;
use yolo_architecture_syntax::targeted_tokens::Token;

const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const ASSIGNMENT: &str = "=";
const LET_KEYWORD: &str = "let";
const MUTABLE_KEYWORD: &str = "mut";
const IF_KEYWORD: &str = "if";
const ERROR_VARIANT: &str = "Err";
const ERROR_BINDING: &str = "error";
const NONE_VARIANT: &str = "None";
const OK_VARIANT: &str = "Ok";
const OBSERVED_BINDING: &str = "observed";
const TERMINATED_BINDING: &str = "terminated";
const REFERENCE: &str = "&";
const ARROW_END: &str = ">";
const PATH_SEPARATOR: &str = "::";
const STANDARD_LIBRARY: &str = "std";
const PROCESS_MODULE: &str = "process";
const ROOTED_CALL_PREFIX: &[&str] = &[
    PATH_SEPARATOR,
    STANDARD_LIBRARY,
    PATH_SEPARATOR,
    PROCESS_MODULE,
    PATH_SEPARATOR,
    contract::CHILD,
    PATH_SEPARATOR,
];
const NEXT_TOKEN_OFFSET: usize = 1;
const ROOTED_CHILD_ARGUMENT_TOKEN_COUNT: usize = 5;
const ARM_BLOCK_OPEN_OFFSET: usize = 6;
const REQUIRED_RESULT_OCCURRENCES: usize = 2;
const TOP_LEVEL_DEPTH: usize = 0;
const NESTING_STEP: usize = 1;

pub fn exact_poll_flow(tokens: &[Token], function: &RustFunction, polls: &[usize]) -> bool {
    polls.iter().all(|poll| {
        exact_rooted_child_call(tokens, *poll, contract::TRY_WAIT)
            && direct_result_assignment(tokens, *poll, OBSERVED_BINDING)
    }) && binding_occurrences(tokens, function, OBSERVED_BINDING) == REQUIRED_RESULT_OCCURRENCES
}

pub fn exact_termination_result(tokens: &[Token], function: &RustFunction, kill: usize) -> bool {
    direct_result_assignment(tokens, kill, TERMINATED_BINDING)
        && binding_occurrences(tokens, function, TERMINATED_BINDING) == REQUIRED_RESULT_OCCURRENCES
}

pub fn exact_wait_guard(tokens: &[Token], wait: usize) -> bool {
    rooted_call_start(wait).is_some_and(|root| {
        sequence::preceding(
            tokens,
            root,
            &[
                IF_KEYWORD,
                LET_KEYWORD,
                ERROR_VARIANT,
                CALL_OPEN,
                ERROR_BINDING,
                CALL_CLOSE,
                ASSIGNMENT,
            ],
        )
    })
}

pub fn exact_rooted_child_call(tokens: &[Token], call: usize, method: &str) -> bool {
    if !tokens[call].identifier(method) || !sequence::preceding(tokens, call, ROOTED_CALL_PREFIX) {
        return false;
    }
    matches!(
        tokens.get(
            call + NEXT_TOKEN_OFFSET
                ..call + NEXT_TOKEN_OFFSET + ROOTED_CHILD_ARGUMENT_TOKEN_COUNT,
        ),
        Some([open, reference, mutable, child, close])
            if open.symbol(CALL_OPEN)
                && reference.symbol(REFERENCE)
                && mutable.identifier(MUTABLE_KEYWORD)
                && child.identifier(contract::CHILD_RECEIVER)
                && close.symbol(CALL_CLOSE)
    )
}

pub fn calls_are_top_level(
    tokens: &[Token],
    range: std::ops::Range<usize>,
    kill: usize,
    wait: usize,
) -> bool {
    range.into_iter().any(|index| {
        tokens[index].identifier(OK_VARIANT)
            && sequence::following(
                tokens,
                index,
                &[
                    CALL_OPEN,
                    NONE_VARIANT,
                    CALL_CLOSE,
                    ASSIGNMENT,
                    ARROW_END,
                    BLOCK_OPEN,
                ],
            )
            && top_level_at(tokens, index + ARM_BLOCK_OPEN_OFFSET, kill)
            && top_level_at(tokens, index + ARM_BLOCK_OPEN_OFFSET, wait)
    })
}

fn direct_result_assignment(tokens: &[Token], call: usize, binding: &str) -> bool {
    rooted_call_start(call)
        .is_some_and(|root| sequence::preceding(tokens, root, &[LET_KEYWORD, binding, ASSIGNMENT]))
}

fn rooted_call_start(call: usize) -> Option<usize> {
    call.checked_sub(ROOTED_CALL_PREFIX.len())
}

fn binding_occurrences(tokens: &[Token], function: &RustFunction, binding: &str) -> usize {
    function
        .body
        .clone()
        .filter(|index| tokens[*index].identifier(binding))
        .count()
}

fn top_level_at(tokens: &[Token], open: usize, position: usize) -> bool {
    let mut depth = TOP_LEVEL_DEPTH;
    for token in &tokens[open + NEXT_TOKEN_OFFSET..position] {
        if token.symbol(BLOCK_OPEN) {
            depth += NESTING_STEP;
        } else if token.symbol(BLOCK_CLOSE) {
            depth = depth.saturating_sub(NESTING_STEP);
        }
    }
    depth == TOP_LEVEL_DEPTH
}
