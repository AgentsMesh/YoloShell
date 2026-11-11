use crate::pty_master_closure_contract as contract;
use yolo_architecture_syntax::targeted_tokens::Token;

const FUNCTION_KEYWORD: &str = "fn";
const PUBLIC_KEYWORD: &str = "pub";
const STRUCT_KEYWORD: &str = "struct";
const IMPL_KEYWORD: &str = "impl";
const ENUM_KEYWORD: &str = "enum";
const LEFT_PAREN: &str = "(";
const RIGHT_PAREN: &str = ")";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const MEMBER_ACCESS: &str = ".";
const PATH_ACCESS: &str = "::";
const STATEMENT_END: &str = ";";
const NEXT_TOKEN_OFFSET: usize = 1;
const PUBLIC_DEFINITION_PREFIX_LENGTH: usize = 2;
const ACK_PREFIX_LENGTH: usize = 4;
const ACK_SUFFIX_LENGTH: usize = 3;
const QUALIFIED_CALL_PREFIX_LENGTH: usize = 2;
const EXPECTED_ONCE: usize = 1;

pub fn method_occurrence(tokens: &[Token], index: usize) -> Option<contract::Occurrence> {
    if previous(tokens, index, FUNCTION_KEYWORD) {
        Some(contract::Occurrence::Definition)
    } else if next(tokens, index, LEFT_PAREN) {
        Some(contract::Occurrence::Call)
    } else {
        None
    }
}

pub fn constructor(tokens: &[Token], index: usize) -> bool {
    next(tokens, index, BLOCK_OPEN) && !item_header(tokens, index)
}

pub fn crate_definition(tokens: &[Token], index: usize) -> bool {
    let Some(start) = index.checked_sub(PUBLIC_DEFINITION_PREFIX_LENGTH) else {
        return false;
    };
    let Some([public, function]) = tokens.get(start..index) else {
        return false;
    };
    public.identifier(PUBLIC_KEYWORD) && function.identifier(FUNCTION_KEYWORD)
}

pub fn route_matches(route: contract::Route, tokens: &[Token], index: usize) -> bool {
    match route {
        contract::Route::Definition => crate_definition(tokens, index),
        contract::Route::LocalClose => local_close(tokens, index),
        contract::Route::HostAcknowledgement => exact_acknowledgement(tokens, index),
        contract::Route::StartupHostAcknowledgement => {
            crate::pty_master_closure_ack_syntax::cloned_verifier_acknowledgement(
                tokens,
                index,
                contract::EXACT_ACK_ARGUMENT,
            )
        }
        contract::Route::HolderGoneAcknowledgement => {
            crate::pty_master_closure_ack_syntax::cloned_verifier_acknowledgement(
                tokens,
                index,
                contract::HOLDER_GONE_ARGUMENT,
            )
        }
        contract::Route::HostSettlement => host_settlement(tokens, index),
        contract::Route::NoSecondaryCustody => no_secondary_custody(tokens, index),
    }
}

pub fn qualified_verifier_acknowledgement(tokens: &[Token], index: usize) -> bool {
    let Some(start) = index.checked_sub(QUALIFIED_CALL_PREFIX_LENGTH) else {
        return false;
    };
    let Some([owner, access]) = tokens.get(start..index) else {
        return false;
    };
    owner.identifier(contract::HOST_VERIFIER) && access.symbol(PATH_ACCESS)
}

fn local_close(tokens: &[Token], index: usize) -> bool {
    let scope = statement(tokens, index);
    count(scope, contract::TAKE_LOCAL) == EXPECTED_ONCE
        && absent(scope, contract::FINALIZE_HOST)
        && absent(scope, contract::FINALIZE_LOCAL)
        && absent(scope, contract::MINT_FINAL)
}

fn exact_acknowledgement(tokens: &[Token], index: usize) -> bool {
    let Some(prefix_start) = index.checked_sub(ACK_PREFIX_LENGTH) else {
        return false;
    };
    let Some(suffix_start) = index.checked_add(NEXT_TOKEN_OFFSET) else {
        return false;
    };
    let Some(suffix_end) = suffix_start.checked_add(ACK_SUFFIX_LENGTH) else {
        return false;
    };
    let Some([receiver, first_access, field, second_access]) = tokens.get(prefix_start..index)
    else {
        return false;
    };
    let Some([left, exact, right]) = tokens.get(suffix_start..suffix_end) else {
        return false;
    };
    receiver.identifier(contract::SELF_RECEIVER)
        && first_access.symbol(MEMBER_ACCESS)
        && field.identifier(contract::VERIFIER_FIELD)
        && second_access.symbol(MEMBER_ACCESS)
        && left.symbol(LEFT_PAREN)
        && exact.identifier(contract::EXACT_ACK_ARGUMENT)
        && right.symbol(RIGHT_PAREN)
}

fn host_settlement(tokens: &[Token], index: usize) -> bool {
    let scope = statement(tokens, index);
    count(scope, contract::FINALIZE_HOST) == EXPECTED_ONCE
        && count(scope, contract::MINT_FINAL) == EXPECTED_ONCE
        && absent(scope, contract::FINALIZE_LOCAL)
        && absent(scope, contract::TAKE_LOCAL)
        && absent(scope, contract::ACKNOWLEDGE)
}

fn no_secondary_custody(tokens: &[Token], index: usize) -> bool {
    let scope = statement(tokens, index);
    count(scope, contract::TAKE_LOCAL) == EXPECTED_ONCE
        && count(scope, contract::FINALIZE_LOCAL) == EXPECTED_ONCE
        && count(scope, contract::MINT_FINAL) == EXPECTED_ONCE
        && absent(scope, contract::FINALIZE_HOST)
        && absent(scope, contract::ACKNOWLEDGE)
}

fn statement(tokens: &[Token], index: usize) -> &[Token] {
    let start = tokens[..index]
        .iter()
        .rposition(|token| token.symbol(STATEMENT_END))
        .map_or(usize::default(), |position| position + NEXT_TOKEN_OFFSET);
    let end = tokens[index..]
        .iter()
        .position(|token| token.symbol(STATEMENT_END))
        .map_or(tokens.len(), |position| index + position);
    &tokens[start..end]
}

fn count(tokens: &[Token], symbol: &str) -> usize {
    tokens
        .iter()
        .filter(|token| token.identifier(symbol))
        .count()
}

fn absent(tokens: &[Token], symbol: &str) -> bool {
    count(tokens, symbol) == usize::default()
}

fn previous(tokens: &[Token], index: usize, value: &str) -> bool {
    index
        .checked_sub(NEXT_TOKEN_OFFSET)
        .and_then(|position| tokens.get(position))
        .is_some_and(|token| token.identifier(value))
}

fn item_header(tokens: &[Token], index: usize) -> bool {
    let start = tokens[..index]
        .iter()
        .rposition(|token| token.symbol(BLOCK_OPEN) || token.symbol(BLOCK_CLOSE))
        .map_or(usize::default(), |position| position + NEXT_TOKEN_OFFSET);
    tokens[start..index].iter().any(|token| {
        [FUNCTION_KEYWORD, STRUCT_KEYWORD, IMPL_KEYWORD, ENUM_KEYWORD]
            .iter()
            .any(|keyword| token.identifier(keyword))
    })
}

fn next(tokens: &[Token], index: usize, value: &str) -> bool {
    let Some(next) = index.checked_add(NEXT_TOKEN_OFFSET) else {
        return false;
    };
    tokens.get(next).is_some_and(|token| token.symbol(value))
}
