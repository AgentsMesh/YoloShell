use super::attributes;
use std::collections::BTreeSet;
use std::ops::Range;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const MODULE_KEYWORD: &str = "mod";
const ITEM_END: &str = ";";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const NAME_OFFSET: usize = 1;
const HEADER_OFFSET: usize = 2;
const RANGE_END_OFFSET: usize = 1;

pub(super) struct Declaration {
    pub index: usize,
    pub name: String,
    pub body: Option<Range<usize>>,
    pub path: Option<String>,
    pub enabled: bool,
    pub test_only: bool,
    pub uncertain: bool,
}

#[derive(Default)]
pub(super) struct Context {
    pub enabled: bool,
    pub test_only: bool,
    pub uncertain: bool,
}

pub(super) fn parse(tokens: &[Token]) -> Vec<Declaration> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.identifier(MODULE_KEYWORD))
        .filter_map(|(index, _)| {
            let name = tokens
                .get(index + NAME_OFFSET)
                .and_then(identifier)?
                .to_owned();
            let boundary = (index + HEADER_OFFSET..tokens.len()).find(|position| {
                tokens[*position].symbol(BLOCK_OPEN) || tokens[*position].symbol(ITEM_END)
            })?;
            let body = tokens[boundary]
                .symbol(BLOCK_OPEN)
                .then(|| closing(tokens, boundary, BLOCK_OPEN, BLOCK_CLOSE))
                .flatten()
                .map(|close| boundary..close + RANGE_END_OFFSET);
            let attributes = attributes::before(tokens, index);
            Some(Declaration {
                index,
                name,
                body,
                path: attributes.path,
                enabled: !attributes.disabled,
                test_only: attributes.test_only,
                uncertain: attributes.uncertain,
            })
        })
        .collect()
}

pub(super) fn unparsed(tokens: &[Token]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        token.identifier(MODULE_KEYWORD)
            && tokens
                .get(index + NAME_OFFSET)
                .and_then(identifier)
                .is_none()
    })
}

pub(super) fn inline_ancestors(tokens: &[Token], index: usize) -> Vec<String> {
    ancestors(&parse(tokens), index)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

pub(super) fn module_item(declarations: &[Declaration], tokens: &[Token], index: usize) -> bool {
    let module_bodies = declarations
        .iter()
        .filter_map(|declaration| declaration.body.as_ref().map(|body| body.start))
        .collect::<BTreeSet<_>>();
    let mut blocks = Vec::new();
    for (position, token) in tokens.iter().enumerate().take(index) {
        if token.symbol(BLOCK_OPEN) {
            blocks.push(position);
        } else if token.symbol(BLOCK_CLOSE) {
            blocks.pop();
        }
    }
    blocks.iter().all(|open| module_bodies.contains(open))
}

pub(super) fn enabled_at(tokens: &[Token], index: usize) -> bool {
    !attributes::before(tokens, index).disabled
}

pub(super) fn test_only(tokens: &[Token], index: usize) -> bool {
    attributes::before(tokens, index).test_only
}

pub(super) fn uncertain_at(tokens: &[Token], index: usize) -> bool {
    attributes::before(tokens, index).uncertain
}

pub(super) fn ancestors(declarations: &[Declaration], index: usize) -> Vec<&str> {
    let mut ancestors = declarations
        .iter()
        .filter(|item| item.body.as_ref().is_some_and(|body| body.contains(&index)))
        .collect::<Vec<_>>();
    ancestors.sort_by_key(|item| item.body.as_ref().map(|body| body.start));
    ancestors
        .into_iter()
        .map(|item| item.name.as_str())
        .collect()
}

pub(super) fn context(declarations: &[Declaration], index: usize) -> Context {
    let ancestors = declarations
        .iter()
        .filter(|item| item.body.as_ref().is_some_and(|body| body.contains(&index)))
        .collect::<Vec<_>>();
    Context {
        enabled: ancestors.iter().all(|item| item.enabled),
        test_only: ancestors.iter().any(|item| item.test_only),
        uncertain: ancestors.iter().any(|item| item.uncertain),
    }
}
