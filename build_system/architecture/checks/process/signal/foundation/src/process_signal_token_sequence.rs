use yolo_architecture_syntax::targeted_tokens::Token;

const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const ATTRIBUTE_OPEN: &str = "[";
const ATTRIBUTE_CLOSE: &str = "]";
const MACRO_MARKER: &str = "!";
const FIRST_CONTENT_OFFSET: usize = 1;
const NESTING_STEP: usize = 1;
const TOP_LEVEL_DEPTH: usize = 0;

pub fn macro_argument(tokens: &[Token], index: usize) -> bool {
    let mut openings = Vec::new();
    for position in 0..index {
        if opening(&tokens[position]).is_some() {
            openings.push(position);
            continue;
        }
        let Some(expected) = closing_open(&tokens[position]) else {
            continue;
        };
        if openings
            .last()
            .is_some_and(|open| tokens[*open].symbol(expected))
        {
            openings.pop();
        }
    }
    openings.into_iter().any(|open| {
        let Some(marker) = open.checked_sub(FIRST_CONTENT_OFFSET) else {
            return false;
        };
        tokens
            .get(marker)
            .is_some_and(|token| token.symbol(MACRO_MARKER))
            && marker
                .checked_sub(FIRST_CONTENT_OFFSET)
                .and_then(|owner| tokens.get(owner))
                .is_some_and(|token| {
                    matches!(
                        &token.kind,
                        yolo_architecture_syntax::targeted_tokens::Kind::Identifier(_)
                    )
                })
    })
}

pub fn contains_top_level_identifier(
    tokens: &[Token],
    open: usize,
    close: usize,
    name: &str,
) -> bool {
    let mut depth = TOP_LEVEL_DEPTH;
    for (offset, token) in tokens[open + FIRST_CONTENT_OFFSET..close]
        .iter()
        .enumerate()
    {
        let index = open + FIRST_CONTENT_OFFSET + offset;
        if token.symbol(BLOCK_OPEN) {
            depth += NESTING_STEP;
        } else if token.symbol(BLOCK_CLOSE) {
            depth = depth.saturating_sub(NESTING_STEP);
        } else if depth == TOP_LEVEL_DEPTH
            && token.identifier(name)
            && !macro_argument(tokens, index)
        {
            return true;
        }
    }
    false
}

pub fn contains_top_level_sequence(
    tokens: &[Token],
    open: usize,
    close: usize,
    names: &[&str],
) -> bool {
    let mut depth = TOP_LEVEL_DEPTH;
    for index in open + FIRST_CONTENT_OFFSET..close {
        if tokens[index].symbol(BLOCK_OPEN) {
            depth += NESTING_STEP;
        } else if tokens[index].symbol(BLOCK_CLOSE) {
            depth = depth.saturating_sub(NESTING_STEP);
        } else if depth == TOP_LEVEL_DEPTH
            && !macro_argument(tokens, index)
            && tokens
                .get(index..index + names.len())
                .is_some_and(|tail| matches(tail, names))
        {
            return true;
        }
    }
    false
}

fn opening(token: &Token) -> Option<&'static str> {
    if token.symbol(CALL_OPEN) {
        Some(CALL_OPEN)
    } else if token.symbol(BLOCK_OPEN) {
        Some(BLOCK_OPEN)
    } else if token.symbol(ATTRIBUTE_OPEN) {
        Some(ATTRIBUTE_OPEN)
    } else {
        None
    }
}

fn closing_open(token: &Token) -> Option<&'static str> {
    if token.symbol(CALL_CLOSE) {
        Some(CALL_OPEN)
    } else if token.symbol(BLOCK_CLOSE) {
        Some(BLOCK_OPEN)
    } else if token.symbol(ATTRIBUTE_CLOSE) {
        Some(ATTRIBUTE_OPEN)
    } else {
        None
    }
}

pub fn preceding(tokens: &[Token], index: usize, names: &[&str]) -> bool {
    index
        .checked_sub(names.len())
        .is_some_and(|start| matches(&tokens[start..index], names))
}

pub fn following(tokens: &[Token], index: usize, names: &[&str]) -> bool {
    tokens
        .get(index + FIRST_CONTENT_OFFSET..index + FIRST_CONTENT_OFFSET + names.len())
        .is_some_and(|tail| matches(tail, names))
}

fn matches(tokens: &[Token], names: &[&str]) -> bool {
    tokens
        .iter()
        .zip(names)
        .all(|(token, name)| token.identifier(name) || token.symbol(name))
}
