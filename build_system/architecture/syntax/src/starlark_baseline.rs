use crate::starlark_scan::{LIST_CLOSE, SEPARATOR, list_start, without_comments};
use crate::targeted_tokens::{Kind, Token, tokenize};

pub use crate::starlark_scan::Malformed;

const TUPLE_OPEN: &str = "(";
const TUPLE_CLOSE: &str = ")";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    pub line: usize,
    pub values: Vec<String>,
}

/// Reads every string tuple assigned to `assignment` in a Starlark file.
///
/// The reader is intentionally strict so a baseline cannot silently stop
/// matching its sources: the assignment must be a literal list of tuples with
/// exactly `arity` string elements, and any other token inside the list is a
/// malformed entry. Starlark `#` comments are ignored.
pub fn tuple_entries(source: &str, assignment: &str, arity: usize) -> Result<Vec<Entry>, Malformed> {
    let tokens = without_comments(tokenize(source));
    let mut index = list_start(&tokens, assignment)?;
    let mut entries = Vec::new();
    loop {
        let token = tokens.get(index).ok_or(Malformed { line: 1 })?;
        if token.symbol(LIST_CLOSE) {
            return Ok(entries);
        }
        if token.symbol(SEPARATOR) {
            index += 1;
            continue;
        }
        if !token.symbol(TUPLE_OPEN) {
            return Err(Malformed { line: token.line });
        }
        let (entry, next) = tuple(&tokens, index, arity)?;
        entries.push(entry);
        index = next;
    }
}

fn tuple(tokens: &[Token], open: usize, arity: usize) -> Result<(Entry, usize), Malformed> {
    let line = tokens[open].line;
    let mut values = Vec::new();
    let mut index = open + 1;
    loop {
        let token = tokens.get(index).ok_or(Malformed { line })?;
        match &token.kind {
            Kind::String(value) => values.push(value.clone()),
            Kind::Symbol(symbol) if symbol == SEPARATOR => {}
            Kind::Symbol(symbol) if symbol == TUPLE_CLOSE => {
                index += 1;
                break;
            }
            _ => return Err(Malformed { line: token.line }),
        }
        index += 1;
    }
    if values.len() != arity {
        return Err(Malformed { line });
    }
    Ok((Entry { line, values }, index))
}
