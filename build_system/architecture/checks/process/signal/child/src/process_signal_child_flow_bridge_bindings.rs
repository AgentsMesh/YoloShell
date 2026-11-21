use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_token_sequence as sequence;
use yolo_architecture_syntax::rust_source::RustFunction;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const ITEM_END: &str = ";";
const ASSIGNMENT: &str = "=";
const LET_KEYWORD: &str = "let";
const FUNCTION_KEYWORD: &str = "fn";
const MUTABLE_KEYWORD: &str = "mut";
const SOME_VARIANT: &str = "Some";
const MACRO_MARKER: &str = "!";
const NEXT_TOKEN_OFFSET: usize = 1;
const EXTRACTION_CALL_OPEN_OFFSET: usize = 2;
const EXTRACTION_BINDING_OFFSET: usize = 4;

pub fn stable_child_binding(tokens: &[Token], function: &RustFunction) -> bool {
    !function.body.clone().any(|index| {
        tokens[index].identifier(contract::CHILD_RECEIVER)
            && (tokens
                .get(index + NEXT_TOKEN_OFFSET)
                .is_some_and(|token| token.symbol(ASSIGNMENT))
                || shadow_binding(tokens, function, index))
    })
}

pub fn no_settlement_shadowing(tokens: &[Token], function: &RustFunction) -> bool {
    function.body.clone().all(|index| {
        !tokens[index].identifier(contract::SETTLE_FAILED_CHILD)
            || tokens
                .get(index.checked_sub(NEXT_TOKEN_OFFSET).unwrap_or(index))
                .is_none_or(|token| {
                    !token.identifier(LET_KEYWORD) && !token.identifier(FUNCTION_KEYWORD)
                })
                && tokens
                    .get(index + NEXT_TOKEN_OFFSET)
                    .is_none_or(|token| !token.symbol(MACRO_MARKER))
    })
}

fn shadow_binding(tokens: &[Token], function: &RustFunction, binding: usize) -> bool {
    let start = tokens[function.body.start..binding]
        .iter()
        .rposition(|token| token.symbol(ITEM_END))
        .map_or(function.body.start, |offset| {
            function.body.start + offset + NEXT_TOKEN_OFFSET
        });
    let Some(let_token) = tokens[start..binding]
        .iter()
        .rposition(|token| token.identifier(LET_KEYWORD))
        .map(|offset| start + offset)
    else {
        return false;
    };
    let statement_end = tokens[binding..function.body.end]
        .iter()
        .position(|token| token.symbol(ITEM_END))
        .map(|offset| binding + offset)
        .unwrap_or(function.body.end);
    let Some(assignment) = tokens[binding..statement_end]
        .iter()
        .position(|token| token.symbol(ASSIGNMENT))
        .map(|offset| binding + offset)
    else {
        return false;
    };
    let_token < binding && binding < assignment && !extraction_binding(tokens, let_token, binding)
}

fn extraction_binding(tokens: &[Token], let_token: usize, binding: usize) -> bool {
    sequence::following(
        tokens,
        let_token,
        &[SOME_VARIANT, CALL_OPEN, MUTABLE_KEYWORD],
    ) && binding == let_token + EXTRACTION_BINDING_OFFSET
        && closing(
            tokens,
            let_token + EXTRACTION_CALL_OPEN_OFFSET,
            CALL_OPEN,
            CALL_CLOSE,
        )
        .is_some()
}
