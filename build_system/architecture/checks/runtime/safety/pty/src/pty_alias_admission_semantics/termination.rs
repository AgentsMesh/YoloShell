use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::constants::{
    PTY_ALIAS_OWNER_SUBMISSION as SUBMIT_TERMINATION, PTY_ALIAS_PERMIT_BINDING as PERMIT_BINDING,
    PTY_ALIAS_TERMINATION_FUNCTION as TERMINATION_FUNCTION,
    PTY_ALIAS_TERMINATION_PERMIT as TERMINATION_PERMIT,
};
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::Token;

const GENERIC_PERMIT: &str = "P";
const DROP: &str = "drop";
const AWAIT: &str = "await";
const LEFT_PAREN: &str = "(";
const RIGHT_PAREN: &str = ")";
const TYPE_SEPARATOR: &str = ":";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const STATEMENT_END: &str = ";";
const POLICY: &str = "PTY alias admission must consume the typed mutation permit after owner command submission and before the first await in each branch";

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    let source = RustSource::parse(&file.text);
    let functions: Vec<_> = source
        .functions()
        .filter(|function| function.matches(None, TERMINATION_FUNCTION))
        .collect();
    let valid = matches!(functions.as_slice(), [function]
        if valid_termination(&source.tokens[function.body.clone()]));
    if !valid {
        violations.push(Violation::file(&file.relative, POLICY));
    }
}

fn valid_termination(tokens: &[Token]) -> bool {
    let submissions = positions(tokens, SUBMIT_TERMINATION);
    let drops = direct_drop_positions(tokens);
    let awaits = positions(tokens, AWAIT);
    typed_permit(tokens)
        && matches!(submissions.as_slice(), [submission]
            if owner_drop_is_ordered(tokens, *submission, &drops, &awaits))
        && !awaits.is_empty()
        && drops
            .iter()
            .all(|drop| !prior_in_same_block(tokens, *drop, &awaits))
        && awaits
            .iter()
            .all(|awaited| prior_in_same_block(tokens, *awaited, &drops))
}

fn typed_permit(tokens: &[Token]) -> bool {
    has_sequence(tokens, GENERIC_PERMIT, TYPE_SEPARATOR, TERMINATION_PERMIT)
        && has_sequence(tokens, PERMIT_BINDING, TYPE_SEPARATOR, GENERIC_PERMIT)
}

fn direct_drop_positions(tokens: &[Token]) -> Vec<usize> {
    (0..tokens.len())
        .filter(|index| exact_drop(tokens, *index))
        .collect()
}

fn exact_drop(tokens: &[Token], index: usize) -> bool {
    let Some(window) = tokens.get(index..index + 4) else {
        return false;
    };
    matches!(window, [drop, left, permit, right]
        if drop.identifier(DROP)
            && left.symbol(LEFT_PAREN)
            && permit.identifier(PERMIT_BINDING)
            && right.symbol(RIGHT_PAREN))
        && tokens
            .get(index + 4)
            .is_some_and(|token| token.symbol(STATEMENT_END))
        && index
            .checked_sub(1)
            .and_then(|prior| tokens.get(prior))
            .is_some_and(|token| token.symbol(STATEMENT_END) || token.symbol(BLOCK_OPEN))
}

fn owner_drop_is_ordered(
    tokens: &[Token],
    submission: usize,
    drops: &[usize],
    awaits: &[usize],
) -> bool {
    drops.iter().any(|drop| {
        submission < *drop
            && block_encloses(tokens, *drop, submission)
            && !awaits
                .iter()
                .any(|awaited| submission < *awaited && *awaited < *drop)
    })
}

fn prior_in_same_block(tokens: &[Token], index: usize, candidates: &[usize]) -> bool {
    let block = enclosing_block(tokens, index);
    candidates
        .iter()
        .any(|candidate| *candidate < index && enclosing_block(tokens, *candidate) == block)
}

fn enclosing_block(tokens: &[Token], index: usize) -> Option<usize> {
    enclosing_blocks(tokens, index).pop()
}

fn block_encloses(tokens: &[Token], outer: usize, inner: usize) -> bool {
    let outer = enclosing_blocks(tokens, outer);
    let inner = enclosing_blocks(tokens, inner);
    inner.starts_with(&outer)
}

fn enclosing_blocks(tokens: &[Token], index: usize) -> Vec<usize> {
    let mut stack = Vec::new();
    for (position, token) in tokens.iter().enumerate().take(index) {
        if token.symbol(BLOCK_OPEN) {
            stack.push(position);
        } else if token.symbol(BLOCK_CLOSE) {
            stack.pop();
        }
    }
    stack
}

fn positions(tokens: &[Token], identifier: &str) -> Vec<usize> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.identifier(identifier))
        .map(|(index, _)| index)
        .collect()
}

fn has_sequence(tokens: &[Token], first: &str, separator: &str, last: &str) -> bool {
    tokens.windows(3).any(|window| {
        window[0].identifier(first) && window[1].symbol(separator) && window[2].identifier(last)
    })
}

#[cfg(all(test, feature = "yolo-runtime-pty-tests"))]
#[path = "../../unit_tests/termination_permit_order_tests.rs"]
mod order_tests;
