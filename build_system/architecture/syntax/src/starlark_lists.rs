use std::collections::BTreeMap;

use crate::starlark_scan::{
    ASSIGNMENT, LIST_CLOSE, SEPARATOR, list_start, without_comments,
};
use crate::targeted_tokens::{Kind, tokenize};

pub use crate::starlark_scan::Malformed;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Item {
    pub line: usize,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Element {
    Name(String),
    String(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementItem {
    pub line: usize,
    pub element: Element,
}

/// Reads the string items assigned to `assignment` in a Starlark file.
pub fn string_entries(source: &str, assignment: &str) -> Result<Vec<Item>, Malformed> {
    let tokens = without_comments(tokenize(source));
    let mut index = list_start(&tokens, assignment)?;
    let mut items = Vec::new();
    loop {
        let token = tokens.get(index).ok_or(Malformed { line: 1 })?;
        match &token.kind {
            Kind::String(value) => items.push(Item {
                line: token.line,
                value: value.clone(),
            }),
            Kind::Symbol(symbol) if symbol == SEPARATOR => {}
            Kind::Symbol(symbol) if symbol == LIST_CLOSE => return Ok(items),
            _ => return Err(Malformed { line: token.line }),
        }
        index += 1;
    }
}

/// Reads a literal list whose items are strings or global names.
///
/// Unlike `string_entries`, this rejects an item that is not followed by a
/// comma or the closing bracket, so a comprehension or call cannot be
/// mistaken for a flat list of names.
pub fn list_entries(source: &str, assignment: &str) -> Result<Vec<ElementItem>, Malformed> {
    let tokens = without_comments(tokenize(source));
    let mut index = list_start(&tokens, assignment)?;
    let mut items = Vec::new();
    loop {
        let token = tokens.get(index).ok_or(Malformed { line: 1 })?;
        if token.symbol(LIST_CLOSE) {
            return Ok(items);
        }
        let element = match &token.kind {
            Kind::String(value) => Element::String(value.clone()),
            Kind::Identifier(name) => Element::Name(name.clone()),
            _ => return Err(Malformed { line: token.line }),
        };
        items.push(ElementItem {
            line: token.line,
            element,
        });
        index += 1;
        match tokens.get(index) {
            Some(next) if next.symbol(SEPARATOR) => index += 1,
            Some(next) if next.symbol(LIST_CLOSE) => {}
            _ => return Err(Malformed { line: token.line }),
        }
    }
}

/// Reads top-level `NAME = "value"` assignments, ignoring comments.
pub fn string_assignments(source: &str) -> BTreeMap<String, String> {
    let tokens = without_comments(tokenize(source));
    let mut assignments = BTreeMap::new();
    for index in 0..tokens.len().saturating_sub(2) {
        let Kind::Identifier(name) = &tokens[index].kind else {
            continue;
        };
        if !name
            .chars()
            .all(|character| character.is_ascii_uppercase() || character == '_')
        {
            continue;
        }
        if !tokens[index + 1].symbol(ASSIGNMENT) {
            continue;
        }
        if let Kind::String(value) = &tokens[index + 2].kind {
            assignments.insert(name.to_owned(), value.clone());
        }
    }
    assignments
}
