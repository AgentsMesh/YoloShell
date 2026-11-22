use super::declarations;
use yolo_architecture_syntax::rust_source::string;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const INCLUDE_MACRO: &str = "include";
const MACRO_MARKER: &str = "!";
const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const MARKER_OFFSET: usize = 1;
const OPEN_OFFSET: usize = 2;
const VALUE_OFFSET: usize = 3;

pub(super) struct Invocation {
    pub index: usize,
    pub literal: Option<String>,
    pub enabled: bool,
    pub test_only: bool,
    pub uncertain: bool,
}

pub(super) fn parse(tokens: &[Token]) -> Vec<Invocation> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.identifier(INCLUDE_MACRO))
        .filter(|(index, _)| {
            tokens
                .get(index + MARKER_OFFSET)
                .is_some_and(|token| token.symbol(MACRO_MARKER))
        })
        .map(|(index, _)| Invocation {
            index,
            literal: literal(tokens, index),
            enabled: declarations::enabled_at(tokens, index),
            test_only: declarations::test_only(tokens, index),
            uncertain: declarations::uncertain_at(tokens, index),
        })
        .collect()
}

fn literal(tokens: &[Token], index: usize) -> Option<String> {
    let open = index + OPEN_OFFSET;
    tokens.get(open)?.symbol(CALL_OPEN).then_some(())?;
    let close = closing(tokens, open, CALL_OPEN, CALL_CLOSE)?;
    let values = tokens.get(index + VALUE_OFFSET..close)?;
    (values.len() == 1)
        .then(|| string(&values[0]).map(str::to_owned))
        .flatten()
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
#[path = "../../unit_tests/include_tests.rs"]
mod tests;
