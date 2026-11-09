use crate::process_signal_authority_sites::CONSTRUCTOR_SITES;
use crate::process_signal_guardian_constants as guardian;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const PATH_SEPARATOR: &str = "::";
const MEMBER_ACCESS: &str = ".";
const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const ARGUMENT_SEPARATOR: &str = ",";
const TRAIT_CAST_KEYWORD: &str = "as";
const TYPE_OPEN: &str = "<";
const TYPE_CLOSE: &str = ">";
const FUNCTION_KEYWORD: &str = "fn";
const STRUCT_KEYWORD: &str = "struct";
const AUTHORITY_BINDING: &str = "authority";
const SELF_BINDING: &str = "self";
const RECEIVER_LOOKBACK: usize = 5;

pub fn sensitive_constructor(qualifier: &str, name: &str) -> bool {
    (qualifier == contract::RETAINED_EXACT_PROCESS
        && (retained_exact_method(name) || retained_exact_observation(name)))
        || CONSTRUCTOR_SITES
            .iter()
            .any(|entry| entry.qualifier == qualifier && entry.name == name)
}

pub fn retained_exact_method(name: &str) -> bool {
    matches!(
        name,
        contract::IS_RUNNING | contract::BEGIN_CONTAINMENT | contract::ENFORCE_CONTAINMENT
    )
}

pub fn retained_exact_observation(name: &str) -> bool {
    name == guardian::OBSERVE_OPERATION
}

pub fn qualified_reference(tokens: &[Token], index: usize) -> Option<(&str, &str)> {
    let name = identifier(tokens.get(index)?)?;
    if index < 2 || !tokens[index - 1].symbol(PATH_SEPARATOR) {
        return None;
    }
    let qualifier =
        identifier(&tokens[index - 2]).or_else(|| angle_qualified_identifier(tokens, index - 2))?;
    Some((qualifier, name))
}

fn angle_qualified_identifier(tokens: &[Token], close: usize) -> Option<&str> {
    if !tokens[close].symbol(TYPE_CLOSE) {
        return None;
    }
    let open = tokens[..close]
        .iter()
        .rposition(|token| token.symbol(TYPE_OPEN))?;
    let qualified = &tokens[open + 1..close];
    if qualified.iter().any(|token| {
        token.symbol(TYPE_OPEN) || token.symbol(TYPE_CLOSE) || token.identifier(TRAIT_CAST_KEYWORD)
    }) {
        return None;
    }
    qualified.iter().rev().find_map(identifier)
}

pub fn tuple_constructor(tokens: &[Token], index: usize) -> Option<&str> {
    let name = identifier(tokens.get(index)?)?;
    if index > 0 && tokens[index - 1].identifier(STRUCT_KEYWORD) {
        return None;
    }
    tokens
        .get(index + 1)
        .is_some_and(|token| token.symbol(CALL_OPEN))
        .then_some(name)
}

pub fn definition(tokens: &[Token], index: usize) -> bool {
    index > 0 && tokens[index - 1].identifier(FUNCTION_KEYWORD)
}

pub fn call(tokens: &[Token], index: usize) -> bool {
    tokens
        .get(index + 1)
        .is_some_and(|token| token.symbol(CALL_OPEN))
}

pub fn method_call(tokens: &[Token], index: usize) -> bool {
    index > 0 && tokens[index - 1].symbol(MEMBER_ACCESS) && call(tokens, index)
}

pub fn call_has_argument_separator(tokens: &[Token], index: usize) -> bool {
    let open = index + 1;
    let Some(close) = closing(tokens, open, CALL_OPEN, CALL_CLOSE) else {
        return false;
    };
    tokens[open + 1..close]
        .iter()
        .any(|token| token.symbol(ARGUMENT_SEPARATOR))
}

pub fn sensitive_receiver(tokens: &[Token], index: usize) -> bool {
    let receiver = &tokens[index.saturating_sub(RECEIVER_LOOKBACK)..index];
    receiver
        .iter()
        .any(|token| token.identifier(AUTHORITY_BINDING))
        || receiver.iter().any(|token| token.identifier(SELF_BINDING))
            && receiver.iter().any(|token| token.number() == Some(0))
}
