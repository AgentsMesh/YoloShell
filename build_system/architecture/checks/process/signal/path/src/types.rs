#[path = "types/declarations.rs"]
mod declarations;
#[path = "types/nesting.rs"]
mod nesting;

use crate::aliases;
use crate::deref;
use crate::paths::{Resolution, VisibilityBoundaries};
use nesting::TypeNesting;
use std::collections::{BTreeMap, BTreeSet};
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const STRUCT_KEYWORD: &str = "struct";
const ASSIGNMENT: &str = "=";
const ITEM_END: &str = ";";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const TYPE_SEPARATOR: &str = ":";
const ARGUMENT_SEPARATOR: &str = ",";
const TUPLE_CLOSE: &str = ")";

#[derive(Default)]
pub struct RetainedTypes {
    names: BTreeSet<String>,
    qualified: BTreeSet<usize>,
    clean_qualified: BTreeSet<usize>,
    canonical_names: BTreeSet<String>,
    canonical_qualified: BTreeSet<usize>,
    aggregate_names: BTreeMap<String, VisibilityBoundaries>,
    aggregate_qualified: BTreeMap<usize, VisibilityBoundaries>,
    declaration_references: BTreeMap<usize, BTreeSet<usize>>,
    declarations: BTreeMap<usize, bool>,
}

impl RetainedTypes {
    pub fn contains(&self, tokens: &[Token], index: usize) -> bool {
        if self.qualified.contains(&index) {
            return true;
        }
        if self.clean_qualified.contains(&index) {
            return false;
        }
        if let Some(declarations) = self.declaration_references.get(&index) {
            return declarations
                .iter()
                .any(|declaration| self.declarations.contains_key(declaration));
        }
        identifier(&tokens[index]).is_some_and(|name| self.names.contains(name))
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn contains_canonical(&self, tokens: &[Token], index: usize) -> bool {
        if self.canonical_qualified.contains(&index) {
            return !self.aggregate_qualified.contains_key(&index);
        }
        if self.aggregate_qualified.contains_key(&index) {
            return false;
        }
        if self.clean_qualified.contains(&index) {
            return false;
        }
        if let Some(declarations) = self.declaration_references.get(&index) {
            return declarations
                .iter()
                .any(|declaration| self.declarations.get(declaration).copied().unwrap_or(false));
        }
        identifier(&tokens[index]).is_some_and(|name| {
            self.canonical_names.contains(name) && !self.aggregate_names.contains_key(name)
        })
    }

    pub fn declaration_canonical(&self, index: usize) -> Option<bool> {
        self.declarations.get(&index).copied()
    }
}

pub fn type_names(tokens: &[Token]) -> RetainedTypes {
    type_names_with(tokens, &Resolution::default())
}

pub fn type_names_with(tokens: &[Token], external: &Resolution) -> RetainedTypes {
    let mut canonical_names = aliases::exact_names(tokens);
    canonical_names.extend(external.canonical_names.iter().cloned());
    while aliases::extend(tokens, &mut canonical_names, &external.canonical_indices) {}
    let mut retained = RetainedTypes {
        names: canonical_names.clone(),
        qualified: external.indices.clone(),
        clean_qualified: external.clean_indices.clone(),
        canonical_names,
        canonical_qualified: external.canonical_indices.clone(),
        aggregate_names: external.aggregate_names.clone(),
        aggregate_qualified: external.aggregate_indices.clone(),
        declaration_references: external.declaration_indices.clone(),
        declarations: BTreeMap::new(),
    };
    retained.names.extend(external.names.iter().cloned());
    loop {
        let mut changed = declarations::extend(tokens, &mut retained);
        changed |= aliases::extend(tokens, &mut retained.names, &retained.qualified);
        for owner in deref::owners(tokens, &retained) {
            changed |= declarations::bind_owner(&mut retained, owner);
        }
        if !changed {
            return retained;
        }
    }
}

pub type RetainedFields = BTreeSet<(String, String)>;

pub fn retained_fields(tokens: &[Token], retained: &RetainedTypes) -> RetainedFields {
    let mut fields = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if !token.identifier(STRUCT_KEYWORD) {
            continue;
        }
        let Some(owner) = tokens.get(index + 1).and_then(identifier) else {
            continue;
        };
        let Some(open) = next_symbol(tokens, index, BLOCK_OPEN) else {
            continue;
        };
        let Some(close) = closing(tokens, open, BLOCK_OPEN, BLOCK_CLOSE) else {
            continue;
        };
        for field in open + 1..close {
            if !tokens[field].symbol(TYPE_SEPARATOR) || field == 0 {
                continue;
            }
            let Some(name) = identifier(&tokens[field - 1]) else {
                continue;
            };
            if type_segment_contains(tokens, field + 1, close, retained) {
                fields.insert((owner.to_owned(), name.to_owned()));
            }
        }
    }
    fields
}

fn next_symbol(tokens: &[Token], start: usize, symbol: &str) -> Option<usize> {
    tokens[start..]
        .iter()
        .position(|token| token.symbol(symbol))
        .map(|offset| start + offset)
}

pub fn type_segment_contains(
    tokens: &[Token],
    start: usize,
    end: usize,
    retained: &RetainedTypes,
) -> bool {
    let mut nesting = TypeNesting::default();
    for (offset, token) in tokens[start..end].iter().enumerate() {
        if nesting.is_top_level() && segment_end(token) {
            break;
        }
        if retained.contains(tokens, start + offset) {
            return true;
        }
        nesting.observe(token);
    }
    false
}

fn segment_end(token: &Token) -> bool {
    token.symbol(ARGUMENT_SEPARATOR)
        || token.symbol(TUPLE_CLOSE)
        || token.symbol(ASSIGNMENT)
        || token.symbol(ITEM_END)
        || token.symbol(BLOCK_CLOSE)
}

#[path = "types/boundaries.rs"]
mod boundaries;

#[cfg(test)]
#[path = "../unit_tests/retained_type_tests.rs"]
mod tests;
