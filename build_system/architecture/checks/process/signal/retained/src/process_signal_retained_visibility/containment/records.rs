use super::scope;
use std::collections::BTreeSet;
use std::ops::Range;
use yolo_architecture_process_signal_foundation::process_signal_retained_export_sites as sites;
use yolo_architecture_build_graph::modules::ModuleIndex;
use yolo_architecture_build_graph::path_syntax::{backward_path, path_candidates};
use yolo_architecture_process_signal_path_analysis::types::RetainedTypes;
use yolo_architecture_process_signal_path_analysis::visibility_fields as syntax;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const AGGREGATE_ITEMS: &[&str] = &["struct", "enum", "union"];
const ITEM_BODY_OFFSET: usize = 2;
const NEXT_TOKEN_OFFSET: usize = 1;
const TRAIT_KEYWORD: &str = "trait";
const UNIQUE_OWNER_COUNT: usize = 1;

pub(super) struct Aggregate {
    pub declaration: usize,
    pub body: Range<usize>,
    pub path: Vec<String>,
    pub visible_from: Vec<String>,
}

pub(super) struct Trait {
    pub path: Vec<String>,
    pub visible_from: Vec<String>,
}

pub(super) fn aggregates(
    relative: &str,
    tokens: &[Token],
    retained: &RetainedTypes,
    modules: &ModuleIndex,
) -> Vec<Aggregate> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| aggregate(token))
        .filter_map(|(index, _)| aggregate_record(relative, tokens, retained, modules, index))
        .collect()
}

pub(super) fn local_names(tokens: &[Token], retained: &RetainedTypes) -> BTreeSet<String> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| aggregate(token))
        .filter_map(|(index, _)| {
            let name_index = index + NEXT_TOKEN_OFFSET;
            let name = tokens.get(name_index).and_then(identifier)?;
            (!sites::requires_exact_type_site(name)
                && retained_body(tokens, retained, index).is_some())
            .then(|| name.to_owned())
        })
        .collect()
}

pub(super) fn traits(relative: &str, tokens: &[Token], modules: &ModuleIndex) -> Vec<Trait> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.identifier(TRAIT_KEYWORD))
        .filter_map(|(index, _)| {
            let name = tokens.get(index + NEXT_TOKEN_OFFSET).and_then(identifier)?;
            let (owner, visible_from) = authority(relative, tokens, modules, index)?;
            let mut path = owner;
            path.push(name.to_owned());
            Some(Trait { path, visible_from })
        })
        .collect()
}

pub(super) fn resolve_aggregate<'a>(
    relative: &str,
    tokens: &[Token],
    index: usize,
    modules: &ModuleIndex,
    aggregates: &'a [Aggregate],
) -> Option<&'a Aggregate> {
    let candidates = candidates(relative, tokens, index, modules)?;
    unique(
        aggregates
            .iter()
            .filter(|record| candidates.contains(&record.path)),
    )
}

pub(super) fn resolve_trait<'a>(
    relative: &str,
    tokens: &[Token],
    index: usize,
    modules: &ModuleIndex,
    traits: &'a [Trait],
) -> Option<&'a Trait> {
    let candidates = candidates(relative, tokens, index, modules)?;
    unique(
        traits
            .iter()
            .filter(|record| candidates.contains(&record.path)),
    )
}

fn aggregate_record(
    relative: &str,
    tokens: &[Token],
    retained: &RetainedTypes,
    modules: &ModuleIndex,
    index: usize,
) -> Option<Aggregate> {
    let name_index = index + NEXT_TOKEN_OFFSET;
    let name = tokens.get(name_index).and_then(identifier)?;
    if sites::requires_exact_type_site(name) {
        return None;
    }
    let body = retained_body(tokens, retained, index)?;
    let (owner, visible_from) = authority(relative, tokens, modules, index)?;
    let mut path = owner;
    path.push(name.to_owned());
    Some(Aggregate {
        declaration: index,
        body,
        path,
        visible_from,
    })
}

fn retained_body(tokens: &[Token], retained: &RetainedTypes, index: usize) -> Option<Range<usize>> {
    let Some((open, left, right)) = syntax::body_delimiter(tokens, index + ITEM_BODY_OFFSET) else {
        return None;
    };
    let Some(close) = closing(tokens, open, left, right) else {
        return None;
    };
    let body = open + NEXT_TOKEN_OFFSET..close;
    body.clone()
        .any(|item| retained.contains(tokens, item))
        .then_some(body)
}

fn authority(
    relative: &str,
    tokens: &[Token],
    modules: &ModuleIndex,
    index: usize,
) -> Option<(Vec<String>, Vec<String>)> {
    if !modules.module_item(relative, index) {
        return None;
    }
    let owners = modules.paths_at(relative, index);
    if owners.len() != UNIQUE_OWNER_COUNT {
        return None;
    }
    let owner = owners.into_iter().next()?;
    let public = syntax::visible_index(tokens, syntax::item_start(tokens, index), index)?;
    let visible_from = scope::restricted(tokens, public, &owner)?;
    Some((owner, visible_from))
}

fn candidates(
    relative: &str,
    tokens: &[Token],
    index: usize,
    modules: &ModuleIndex,
) -> Option<BTreeSet<Vec<String>>> {
    let owners = modules.paths_at(relative, index);
    if owners.len() != UNIQUE_OWNER_COUNT {
        return None;
    }
    let raw = backward_path(tokens, index);
    let owner = owners.into_iter().next()?;
    Some(path_candidates(&raw, &owner).into_iter().collect())
}

fn unique<T>(mut matches: impl Iterator<Item = T>) -> Option<T> {
    let value = matches.next()?;
    matches.next().is_none().then_some(value)
}

fn aggregate(token: &Token) -> bool {
    AGGREGATE_ITEMS
        .iter()
        .any(|keyword| token.identifier(keyword))
}
