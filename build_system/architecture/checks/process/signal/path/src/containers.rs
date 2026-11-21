use crate::types::RetainedTypes;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const STRUCT_KEYWORD: &str = "struct";
const ENUM_KEYWORD: &str = "enum";
const UNION_KEYWORD: &str = "union";
const WHERE_KEYWORD: &str = "where";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const GROUP_OPEN: &str = "(";
const GROUP_CLOSE: &str = ")";
const ITEM_END: &str = ";";
const TYPE_OPEN: &str = "<";
const TYPE_CLOSE: &str = ">";
const TYPE_DOUBLE_CLOSE: &str = ">>";
const ITEM_BODY_OFFSET: usize = 2;

pub fn declarations(tokens: &[Token], retained: &RetainedTypes) -> Vec<(usize, String)> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| item_keyword(token))
        .filter_map(|(index, _)| {
            let name = tokens.get(index + 1).and_then(identifier)?;
            let mut body = item_body(tokens, index + ITEM_BODY_OFFSET)?;
            body.any(|(body_index, _)| retained.contains(tokens, body_index))
                .then(|| (index, name.to_owned()))
        })
        .collect()
}

fn item_keyword(token: &Token) -> bool {
    token.identifier(STRUCT_KEYWORD)
        || token.identifier(ENUM_KEYWORD)
        || token.identifier(UNION_KEYWORD)
}

fn item_body(tokens: &[Token], start: usize) -> Option<impl Iterator<Item = (usize, &Token)>> {
    let (open, left, right) = body_delimiter(tokens, start)?;
    let close = closing(tokens, open, left, right)?;
    Some(
        tokens[open + 1..close]
            .iter()
            .enumerate()
            .map(move |(offset, token)| (open + 1 + offset, token)),
    )
}

fn body_delimiter(tokens: &[Token], start: usize) -> Option<(usize, &'static str, &'static str)> {
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
