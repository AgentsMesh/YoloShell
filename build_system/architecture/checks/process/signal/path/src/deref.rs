use crate::types::RetainedTypes;
use std::collections::BTreeSet;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const IMPL_KEYWORD: &str = "impl";
const FOR_KEYWORD: &str = "for";
const WHERE_KEYWORD: &str = "where";
const TYPE_KEYWORD: &str = "type";
const TARGET_TYPE: &str = "Target";
const DEREF_TRAIT: &str = "Deref";
const DEREF_MUT_TRAIT: &str = "DerefMut";
const AS_KEYWORD: &str = "as";
const ASSIGNMENT: &str = "=";
const ITEM_END: &str = ";";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const TYPE_OPEN: &str = "<";
const TYPE_CLOSE: &str = ">";
const TYPE_DOUBLE_CLOSE: &str = ">>";

pub struct Owner {
    pub name: String,
    pub index: usize,
}

pub fn owners(tokens: &[Token], retained: &RetainedTypes) -> Vec<Owner> {
    let (deref, deref_mut) = trait_names(tokens);
    impls(tokens)
        .filter_map(|implementation| {
            let trait_name = implementation.trait_name?;
            let sensitive = (deref.contains(trait_name)
                && target_is_retained(implementation.body, retained))
                || (deref_mut.contains(trait_name)
                    && retained.contains(tokens, implementation.owner_index));
            sensitive.then(|| Owner {
                name: implementation.owner.to_owned(),
                index: implementation.owner_index,
            })
        })
        .collect()
}

pub fn inspect(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    violations: &mut Vec<Violation>,
) {
    let (deref, deref_mut) = trait_names(&source.tokens);
    for implementation in impls(&source.tokens) {
        let Some(trait_name) = implementation.trait_name else {
            continue;
        };
        if (deref.contains(trait_name) && target_is_retained(implementation.body, retained))
            || (deref_mut.contains(trait_name)
                && retained.contains(&source.tokens, implementation.owner_index))
        {
            violations.push(Violation::line(
                &file.relative,
                implementation.line,
                contract::AUTHORITY_TYPE_MESSAGE,
            ));
        }
    }
}

struct Implementation<'a> {
    trait_name: Option<&'a str>,
    owner: &'a str,
    owner_index: usize,
    body: &'a [Token],
    line: usize,
}

fn impls(tokens: &[Token]) -> impl Iterator<Item = Implementation<'_>> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.identifier(IMPL_KEYWORD))
        .filter_map(|(index, token)| {
            let open = tokens[index + 1..]
                .iter()
                .position(|candidate| candidate.symbol(BLOCK_OPEN))?
                + index
                + 1;
            let close = closing(tokens, open, BLOCK_OPEN, BLOCK_CLOSE)?;
            let header = &tokens[index + 1..open];
            let separator = header
                .iter()
                .position(|candidate| candidate.identifier(FOR_KEYWORD));
            let trait_name =
                separator.and_then(|end| outer_type_name(&header[..end]).map(|(_, name)| name));
            let owner_start = separator.map_or(0, |position| position + 1);
            let (owner_offset, owner) = outer_type_name(&header[owner_start..])?;
            Some(Implementation {
                trait_name,
                owner,
                owner_index: index + 1 + owner_start + owner_offset,
                body: &tokens[open + 1..close],
                line: token.line,
            })
        })
}

fn target_is_retained(tokens: &[Token], retained: &RetainedTypes) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        if !token.identifier(TYPE_KEYWORD)
            || !tokens
                .get(index + 1)
                .is_some_and(|candidate| candidate.identifier(TARGET_TYPE))
        {
            return false;
        }
        let end = tokens[index..]
            .iter()
            .position(|candidate| candidate.symbol(ITEM_END))
            .map_or(tokens.len(), |offset| index + offset);
        let Some(assignment) = tokens[index..end]
            .iter()
            .position(|candidate| candidate.symbol(ASSIGNMENT))
            .map(|offset| index + offset)
        else {
            return false;
        };
        tokens[assignment + 1..end]
            .iter()
            .enumerate()
            .any(|(offset, _)| retained.contains(tokens, assignment + 1 + offset))
    })
}

fn outer_type_name(tokens: &[Token]) -> Option<(usize, &str)> {
    let mut depth = 0_usize;
    let mut name = None;
    for (index, token) in tokens.iter().enumerate() {
        if token.symbol(TYPE_OPEN) {
            depth += 1;
        } else if token.symbol(TYPE_CLOSE) {
            depth = depth.saturating_sub(1);
        } else if token.symbol(TYPE_DOUBLE_CLOSE) {
            depth = depth.saturating_sub(2);
        } else if depth == 0 && token.identifier(WHERE_KEYWORD) {
            break;
        } else if depth == 0
            && let Some(candidate) = identifier(token)
        {
            name = Some((index, candidate));
        }
    }
    name
}

fn trait_names(tokens: &[Token]) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut deref = BTreeSet::from([DEREF_TRAIT.to_owned()]);
    let mut deref_mut = BTreeSet::from([DEREF_MUT_TRAIT.to_owned()]);
    loop {
        let mut changed = false;
        for (index, token) in tokens.iter().enumerate() {
            if !token.identifier(AS_KEYWORD) || index == 0 {
                continue;
            }
            let Some(source) = tokens[..index].iter().rev().find_map(identifier) else {
                continue;
            };
            let Some(alias) = tokens.get(index + 1).and_then(identifier) else {
                continue;
            };
            changed |= deref.contains(source) && deref.insert(alias.to_owned());
            changed |= deref_mut.contains(source) && deref_mut.insert(alias.to_owned());
        }
        if !changed {
            return (deref, deref_mut);
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/deref_shape_tests.rs"]
mod tests;
