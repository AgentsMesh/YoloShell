use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_token_sequence as sequence;
use yolo_architecture_syntax::rust_source::RustFunction;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const TYPE_SEPARATOR: &str = ":";
const TYPE_OPEN: &str = "<";
const TYPE_CLOSE: &str = ">";
const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const ASSIGNMENT: &str = "=";
const MEMBER_ACCESS: &str = ".";
const TRY_OPERATOR: &str = "?";
const ITEM_END: &str = ";";
const BLOCK_OPEN: &str = "{";
const LET_KEYWORD: &str = "let";
const OPTION_TYPE: &str = "Option";
const CONTEXT_METHOD: &str = "context";
const MACRO_MARKER: &str = "!";
const NEXT_TOKEN_OFFSET: usize = 1;
const CONTEXT_CALL_OPEN_OFFSET: usize = 5;
const CALL_TRAILER_TOKEN_COUNT: usize = 2;
const REQUIRED_BINDING_OCCURRENCES: usize = 2;

pub fn exact_bindings(tokens: &[Token], wrapper: &RustFunction) -> bool {
    let inputs: Vec<_> = occurrences(tokens, wrapper, contract::AUDIT_INPUT_BINDING);
    let calls: Vec<_> = occurrences(tokens, wrapper, contract::AUDIT_CALL_BINDING);
    inputs.len() == REQUIRED_BINDING_OCCURRENCES
        && calls.len() == REQUIRED_BINDING_OCCURRENCES
        && inputs.iter().all(|index| {
            input_parameter(tokens, wrapper, *index) || resolved_source(tokens, *index)
        })
        && calls
            .iter()
            .all(|index| resolved_binding(tokens, *index) || direct_call(tokens, *index))
        && !wrapper
            .body
            .clone()
            .any(|index| tokens[index].symbol(MACRO_MARKER))
}

fn input_parameter(tokens: &[Token], wrapper: &RustFunction, input: usize) -> bool {
    let Some(body_open) = wrapper
        .body
        .clone()
        .find(|index| tokens[*index].symbol(BLOCK_OPEN))
    else {
        return false;
    };
    input < body_open
        && sequence::following(
            tokens,
            input,
            &[
                TYPE_SEPARATOR,
                OPTION_TYPE,
                TYPE_OPEN,
                contract::AUDIT_SIGNAL_FUNCTION,
                TYPE_CLOSE,
            ],
        )
}

fn resolved_binding(tokens: &[Token], binding: usize) -> bool {
    sequence::preceding(tokens, binding, &[LET_KEYWORD])
        && sequence::following(
            tokens,
            binding,
            &[
                ASSIGNMENT,
                contract::AUDIT_INPUT_BINDING,
                MEMBER_ACCESS,
                CONTEXT_METHOD,
                CALL_OPEN,
            ],
        )
        && exact_context_trailer(tokens, binding)
}

fn resolved_source(tokens: &[Token], input: usize) -> bool {
    sequence::preceding(
        tokens,
        input,
        &[LET_KEYWORD, contract::AUDIT_CALL_BINDING, ASSIGNMENT],
    ) && sequence::following(tokens, input, &[MEMBER_ACCESS, CONTEXT_METHOD, CALL_OPEN])
}

fn exact_context_trailer(tokens: &[Token], binding: usize) -> bool {
    let open = binding + CONTEXT_CALL_OPEN_OFFSET;
    let Some(close) = closing(tokens, open, CALL_OPEN, CALL_CLOSE) else {
        return false;
    };
    matches!(
        tokens.get(close + NEXT_TOKEN_OFFSET..close + NEXT_TOKEN_OFFSET + CALL_TRAILER_TOKEN_COUNT),
        Some([try_operator, item_end])
            if try_operator.symbol(TRY_OPERATOR) && item_end.symbol(ITEM_END)
    )
}

fn direct_call(tokens: &[Token], binding: usize) -> bool {
    tokens
        .get(binding + NEXT_TOKEN_OFFSET)
        .is_some_and(|token| token.symbol(CALL_OPEN))
}

fn occurrences(tokens: &[Token], function: &RustFunction, name: &str) -> Vec<usize> {
    function
        .body
        .clone()
        .filter(|index| tokens[*index].identifier(name))
        .collect()
}

#[cfg(test)]
#[path = "../unit_tests/audit/audit_wrapper_token_tests.rs"]
mod token_tests;
