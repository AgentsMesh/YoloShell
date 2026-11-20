use crate::targeted_tokens::{Kind, Token, closing, tokenize};
use std::ops::Range;

const FN_KEYWORD: &str = "fn";
const IMPL_KEYWORD: &str = "impl";
const FOR_KEYWORD: &str = "for";
const WHERE_KEYWORD: &str = "where";
const TYPE_OPEN: &str = "<";
const TYPE_CLOSE: &str = ">";
const TYPE_DOUBLE_CLOSE: &str = ">>";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const ITEM_END: &str = ";";

pub struct RustSource {
    pub tokens: Vec<Token>,
    functions: Vec<RustFunction>,
}

pub struct RustFunction {
    pub name: String,
    pub owner: Option<String>,
    pub line: usize,
    pub body: Range<usize>,
    attributes: Range<usize>,
}

struct ImplBlock {
    owner: String,
    pub body: Range<usize>,
}

impl RustSource {
    pub fn parse(source: &str) -> Self {
        let tokens = tokenize(source);
        let implementations = impl_blocks(&tokens);
        let functions = functions(&tokens, &implementations);
        Self { tokens, functions }
    }

    pub fn function_at(&self, index: usize) -> Option<&RustFunction> {
        self.functions
            .iter()
            .filter(|function| function.body.contains(&index))
            .min_by_key(|function| function.body.len())
    }

    pub fn functions(&self) -> impl Iterator<Item = &RustFunction> {
        self.functions.iter()
    }

    pub fn is_test_guarded(&self, function: &RustFunction) -> bool {
        super::rust_source_cfg::is_test_guarded(&self.tokens[function.attributes.clone()])
    }

    pub fn is_unit_test_guarded(&self, function: &RustFunction) -> bool {
        super::rust_source_cfg::is_unit_test_guarded(&self.tokens[function.attributes.clone()])
    }
}

impl RustFunction {
    pub fn matches(&self, owner: Option<&str>, name: &str) -> bool {
        self.owner.as_deref() == owner && self.name == name
    }
}

pub fn identifier(token: &Token) -> Option<&str> {
    match &token.kind {
        Kind::Identifier(value) => Some(value),
        _ => None,
    }
}

pub fn string(token: &Token) -> Option<&str> {
    match &token.kind {
        Kind::String(value) => Some(value),
        _ => None,
    }
}

fn impl_blocks(tokens: &[Token]) -> Vec<ImplBlock> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.identifier(IMPL_KEYWORD))
        .filter_map(|(index, _)| {
            let open = next_symbol(tokens, index + 1, BLOCK_OPEN)?;
            let close = closing(tokens, open, BLOCK_OPEN, BLOCK_CLOSE)?;
            let owner = impl_owner(&tokens[index + 1..open])?;
            Some(ImplBlock {
                owner: owner.to_owned(),
                body: open..close + 1,
            })
        })
        .collect()
}

fn impl_owner(tokens: &[Token]) -> Option<&str> {
    let start = top_level_for(tokens).map_or(0, |index| index + 1);
    let mut angle_depth = 0_usize;
    let mut owner = None;
    for token in &tokens[start..] {
        if token.symbol(TYPE_OPEN) {
            angle_depth += 1;
        } else if token.symbol(TYPE_CLOSE) {
            angle_depth = angle_depth.saturating_sub(1);
        } else if token.symbol(TYPE_DOUBLE_CLOSE) {
            angle_depth = angle_depth.saturating_sub(2);
        } else if angle_depth == 0 && token.identifier(WHERE_KEYWORD) {
            break;
        } else if angle_depth == 0
            && let Some(name) = identifier(token)
        {
            owner = Some(name);
        }
    }
    owner
}

fn top_level_for(tokens: &[Token]) -> Option<usize> {
    let mut angle_depth = 0_usize;
    let mut separator = None;
    for (index, token) in tokens.iter().enumerate() {
        if token.symbol(TYPE_OPEN) {
            angle_depth += 1;
        } else if token.symbol(TYPE_CLOSE) {
            angle_depth = angle_depth.saturating_sub(1);
        } else if token.symbol(TYPE_DOUBLE_CLOSE) {
            angle_depth = angle_depth.saturating_sub(2);
        } else if angle_depth == 0 && token.identifier(WHERE_KEYWORD) {
            break;
        } else if angle_depth == 0 && token.identifier(FOR_KEYWORD) {
            separator.get_or_insert(index);
        }
    }
    separator
}

fn functions(tokens: &[Token], implementations: &[ImplBlock]) -> Vec<RustFunction> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.identifier(FN_KEYWORD))
        .filter_map(|(index, token)| {
            let name = tokens.get(index + 1).and_then(identifier)?.to_owned();
            let end = function_end(tokens, index + 2)?;
            let owner = implementations
                .iter()
                .filter(|implementation| implementation.body.contains(&index))
                .min_by_key(|implementation| implementation.body.len())
                .map(|implementation| implementation.owner.clone());
            Some(RustFunction {
                name,
                owner,
                line: token.line,
                body: index..end,
                attributes: item_prefix(tokens, index)..index,
            })
        })
        .collect()
}

fn function_end(tokens: &[Token], start: usize) -> Option<usize> {
    let boundary = tokens[start..]
        .iter()
        .position(|token| token.symbol(BLOCK_OPEN) || token.symbol(ITEM_END))?
        + start;
    if tokens[boundary].symbol(ITEM_END) {
        return Some(boundary + 1);
    }
    closing(tokens, boundary, BLOCK_OPEN, BLOCK_CLOSE).map(|end| end + 1)
}

fn item_prefix(tokens: &[Token], index: usize) -> usize {
    (0..index)
        .rev()
        .find(|position| {
            tokens[*position].symbol(ITEM_END)
                || tokens[*position].symbol(BLOCK_OPEN)
                || tokens[*position].symbol(BLOCK_CLOSE)
        })
        .map_or(0, |position| position + 1)
}

fn next_symbol(tokens: &[Token], start: usize, symbol: &str) -> Option<usize> {
    tokens[start..]
        .iter()
        .position(|token| token.symbol(symbol))
        .map(|offset| start + offset)
}
