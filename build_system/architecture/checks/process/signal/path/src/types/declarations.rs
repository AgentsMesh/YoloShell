use super::RetainedTypes;
use crate::containers;
use crate::deref::Owner;
use crate::visibility_fields as syntax;
use std::collections::btree_map::Entry;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::Token;

const ASSIGNMENT: &str = "=";
const NEXT_TOKEN_OFFSET: usize = 1;
const TYPE_ITEMS: &[&str] = &["enum", "struct", "type", "union"];
const TYPE_KEYWORD: &str = "type";

pub(super) fn extend(tokens: &[Token], retained: &mut RetainedTypes) -> bool {
    let mut changed = false;
    for (index, token) in tokens.iter().enumerate() {
        if !TYPE_ITEMS.iter().any(|keyword| token.identifier(keyword)) {
            continue;
        }
        let Some(name) = tokens.get(index + NEXT_TOKEN_OFFSET).and_then(identifier) else {
            continue;
        };
        if name == contract::RETAINED_EXACT_PROCESS {
            changed |= bind(retained, index, name, true);
        }
        if token.identifier(TYPE_KEYWORD)
            && let Some(canonical) = alias_kind(tokens, retained, index)
        {
            changed |= bind(retained, index, name, canonical);
        }
    }
    for (index, name) in containers::declarations(tokens, retained) {
        changed |= bind(retained, index, &name, false);
    }
    changed
}

pub(super) fn bind_owner(retained: &mut RetainedTypes, owner: Owner) -> bool {
    let declarations = retained
        .declaration_references
        .get(&owner.index)
        .cloned()
        .unwrap_or_default();
    if declarations.is_empty() {
        return retained.names.insert(owner.name);
    }
    declarations.into_iter().fold(false, |changed, index| {
        bind(retained, index, &owner.name, false) || changed
    })
}

fn alias_kind(tokens: &[Token], retained: &RetainedTypes, index: usize) -> Option<bool> {
    let end = syntax::statement_end(tokens, index, tokens.len());
    let assignment = (index..end).find(|candidate| tokens[*candidate].symbol(ASSIGNMENT))?;
    let values = assignment + NEXT_TOKEN_OFFSET..end;
    if values
        .clone()
        .any(|candidate| retained.contains_canonical(tokens, candidate))
    {
        return Some(true);
    }
    values
        .clone()
        .any(|candidate| retained.contains(tokens, candidate))
        .then_some(false)
}

fn bind(retained: &mut RetainedTypes, index: usize, name: &str, canonical: bool) -> bool {
    let mut changed = retained.names.insert(name.to_owned());
    if canonical {
        changed |= retained.canonical_names.insert(name.to_owned());
    }
    match retained.declarations.entry(index) {
        Entry::Vacant(entry) => {
            entry.insert(canonical);
            true
        }
        Entry::Occupied(mut entry) if canonical && !*entry.get() => {
            entry.insert(true);
            true
        }
        Entry::Occupied(_) => changed,
    }
}
