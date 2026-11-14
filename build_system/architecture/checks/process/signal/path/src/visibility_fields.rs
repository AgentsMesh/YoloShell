#[path = "visibility_fields/nesting.rs"]
mod nesting;

use crate::types::RetainedTypes;
use nesting::Nesting;
use std::ops::Range;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const PUBLIC_KEYWORD: &str = "pub";
const PRIVATE_SCOPE: &str = "self";
const RESTRICTED_SCOPE: &str = "in";
const STRUCT_KEYWORD: &str = "struct";
const UNION_KEYWORD: &str = "union";
const WHERE_KEYWORD: &str = "where";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const GROUP_OPEN: &str = "(";
const GROUP_CLOSE: &str = ")";
const TYPE_OPEN: &str = "<";
const TYPE_CLOSE: &str = ">";
const TYPE_DOUBLE_CLOSE: &str = ">>";
const ITEM_END: &str = ";";
const FIELD_SEPARATOR: &str = ",";
const ITEM_BODY_OFFSET: usize = 2;
const NEXT_TOKEN_OFFSET: usize = 1;

pub struct Field {
    pub range: Range<usize>,
    pub public: Option<usize>,
}

pub fn inspect(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    allows: &dyn Fn(usize, usize, Range<usize>) -> bool,
    violations: &mut Vec<Violation>,
) {
    inspect_fields(file, &source.tokens, retained, allows, violations);
}

pub fn item_visible(tokens: &[Token], index: usize) -> bool {
    visible_token(tokens, item_start(tokens, index), index).is_some()
}

pub fn visible_token(tokens: &[Token], start: usize, end: usize) -> Option<&Token> {
    visible_index(tokens, start, end).map(|index| &tokens[index])
}

pub fn visible_index(tokens: &[Token], start: usize, end: usize) -> Option<usize> {
    (start..end).find(|index| {
        tokens[*index].identifier(PUBLIC_KEYWORD) && visibility_expands(tokens, *index, end)
    })
}

pub fn function_header(tokens: &[Token], function: usize) -> Option<Range<usize>> {
    let end = tokens[function..]
        .iter()
        .position(|token| token.symbol(BLOCK_OPEN) || token.symbol(ITEM_END))?
        + function;
    Some(item_start(tokens, function)..end + NEXT_TOKEN_OFFSET)
}

pub fn statement_end(tokens: &[Token], start: usize, limit: usize) -> usize {
    let mut nesting = Nesting::default();
    for (index, token) in tokens.iter().enumerate().take(limit).skip(start) {
        if nesting.is_top_level() && token.symbol(ITEM_END) {
            return index;
        }
        nesting.observe(token);
    }
    limit
}

fn inspect_fields(
    file: &SourceFile,
    tokens: &[Token],
    retained: &RetainedTypes,
    allows: &dyn Fn(usize, usize, Range<usize>) -> bool,
    violations: &mut Vec<Violation>,
) {
    for (index, token) in tokens.iter().enumerate() {
        if !token.identifier(STRUCT_KEYWORD) && !token.identifier(UNION_KEYWORD) {
            continue;
        }
        let Some(fields) = fields(tokens, index) else {
            continue;
        };
        for field in fields {
            if field
                .range
                .clone()
                .any(|candidate| retained.contains(tokens, candidate))
                && let Some(public) = field.public
                && !allows(index, public, field.range)
            {
                violation(file, &tokens[public], violations);
            }
        }
    }
}

pub fn fields(tokens: &[Token], declaration: usize) -> Option<Vec<Field>> {
    let (open, left, right) = body_delimiter(tokens, declaration + ITEM_BODY_OFFSET)?;
    let close = closing(tokens, open, left, right)?;
    let mut result = Vec::new();
    let mut start = open + NEXT_TOKEN_OFFSET;
    while start < close {
        let end = segment_end(tokens, start, close, FIELD_SEPARATOR);
        result.push(Field {
            range: start..end,
            public: visible_index(tokens, start, end),
        });
        start = end.saturating_add(NEXT_TOKEN_OFFSET);
    }
    Some(result)
}

pub fn item_start(tokens: &[Token], index: usize) -> usize {
    (0..index)
        .rev()
        .find(|position| {
            tokens[*position].symbol(ITEM_END)
                || tokens[*position].symbol(BLOCK_OPEN)
                || tokens[*position].symbol(BLOCK_CLOSE)
        })
        .map_or(0, |position| position + NEXT_TOKEN_OFFSET)
}

fn visibility_expands(tokens: &[Token], public: usize, limit: usize) -> bool {
    let restricted = tokens
        .get(public + NEXT_TOKEN_OFFSET)
        .is_some_and(|token| token.symbol(GROUP_OPEN));
    if !restricted {
        return true;
    }
    let open = public + NEXT_TOKEN_OFFSET;
    let close = closing(tokens, open, GROUP_OPEN, GROUP_CLOSE).unwrap_or(limit);
    let mut names = tokens[open + NEXT_TOKEN_OFFSET..close]
        .iter()
        .filter_map(identifier);
    match names.next() {
        Some(PRIVATE_SCOPE) => false,
        Some(RESTRICTED_SCOPE) => names.next() != Some(PRIVATE_SCOPE),
        _ => true,
    }
}

fn segment_end(tokens: &[Token], start: usize, close: usize, delimiter: &str) -> usize {
    let mut nesting = Nesting::default();
    for (index, token) in tokens.iter().enumerate().take(close).skip(start) {
        if nesting.is_top_level() && token.symbol(delimiter) {
            return index;
        }
        nesting.observe(token);
    }
    close
}

pub fn body_delimiter(
    tokens: &[Token],
    start: usize,
) -> Option<(usize, &'static str, &'static str)> {
    let mut generic = 0_usize;
    let mut where_clause = false;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token.symbol(TYPE_OPEN) {
            generic += 1;
        } else if token.symbol(TYPE_CLOSE) {
            generic = generic.saturating_sub(1);
        } else if token.symbol(TYPE_DOUBLE_CLOSE) {
            generic = generic.saturating_sub(2);
        } else if generic == 0 && token.identifier(WHERE_KEYWORD) {
            where_clause = true;
        } else if generic == 0 && token.symbol(BLOCK_OPEN) {
            return Some((index, BLOCK_OPEN, BLOCK_CLOSE));
        } else if generic == 0 && !where_clause && token.symbol(GROUP_OPEN) {
            return Some((index, GROUP_OPEN, GROUP_CLOSE));
        } else if generic == 0 && token.symbol(ITEM_END) {
            return None;
        }
    }
    None
}

#[rustfmt::skip]
pub fn violation(file: &SourceFile, token: &Token, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(&file.relative, token.line, contract::AUTHORITY_TYPE_MESSAGE));
}
