#[path = "imports/globs.rs"]
mod globs;
#[path = "imports/syntax.rs"]
mod syntax;

use super::aliases::{Aliases, Scope};
use super::{Catalog, Resolution};
use std::ops::Range;
use yolo_architecture_process_signal_catalog::Match;
use yolo_architecture_build_graph::modules::ModuleIndex;
use yolo_architecture_build_graph::path_syntax::{BLOCK_CLOSE, BLOCK_OPEN};
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::Token;

const ALIAS_KEYWORD: &str = "as";
const SELF_COMPONENT: &str = "self";
const ITEM_SEPARATOR: &str = ",";
const GLOB: &str = "*";
const FIRST_IMPORT_TOKEN_OFFSET: usize = 1;

pub(super) fn scan(
    tokens: &[Token],
    relative: &str,
    range: Range<usize>,
    scopes: &[Scope],
    catalog: &Catalog,
    modules: &ModuleIndex,
    aliases: &mut Aliases,
    result: &mut Resolution,
) -> bool {
    let mut path = Vec::new();
    let mut groups = Vec::new();
    let mut changed = false;
    let mut skip_alias = false;
    let module_item = modules.module_item(relative, range.start);
    let visibility_index = range.start;
    for index in range.start + FIRST_IMPORT_TOKEN_OFFSET..range.end {
        let token = &tokens[index];
        if token.symbol(BLOCK_OPEN) {
            groups.push(path.clone());
        } else if token.symbol(ITEM_SEPARATOR) {
            path = groups.last().cloned().unwrap_or_default();
            skip_alias = false;
        } else if token.symbol(BLOCK_CLOSE) {
            path = groups.pop().unwrap_or_default();
        } else if token.symbol(GLOB) {
            changed |= globs::import(
                tokens,
                index,
                &path,
                scopes,
                catalog,
                modules,
                aliases,
                result,
                module_item,
                visibility_index,
            );
        } else if let Some(name) = identifier(token) {
            if skip_alias {
                skip_alias = false;
                continue;
            }
            if name == ALIAS_KEYWORD {
                skip_alias = true;
                continue;
            }
            let candidate = if name == SELF_COMPONENT {
                path.clone()
            } else {
                path.push(name.to_owned());
                path.clone()
            };
            if syntax::target(tokens, index, range.end) {
                changed |= import_target_path(
                    tokens,
                    index,
                    &candidate,
                    scopes,
                    catalog,
                    modules,
                    aliases,
                    result,
                    module_item,
                    visibility_index,
                );
            }
        }
    }
    changed
}

fn import_target_path(
    tokens: &[Token],
    index: usize,
    raw: &[String],
    scopes: &[Scope],
    catalog: &Catalog,
    modules: &ModuleIndex,
    aliases: &mut Aliases,
    result: &mut Resolution,
    module_item: bool,
    visibility_index: usize,
) -> bool {
    let name = syntax::binding_name(tokens, index, raw);
    let mut changed = false;
    for scope in scopes {
        for target in aliases.candidates(raw, std::slice::from_ref(scope)) {
            let type_match = catalog.contains(&target, &scope.module);
            match type_match {
                matched @ (Match::Aggregate | Match::Canonical) => {
                    let canonical = matched == Match::Canonical;
                    let boundaries = (!canonical)
                        .then(|| catalog.aggregate_boundaries(&target, &scope.module))
                        .flatten();
                    changed |=
                        result.bind_type(index, name.clone(), canonical, boundaries.as_ref());
                    changed |= aliases.insert(scope, name.clone(), target.clone());
                    if module_item {
                        let visible_from =
                            super::visibility::scope(tokens, visibility_index, &scope.module);
                        changed |= result.bind_type_alias(
                            index,
                            name.clone(),
                            target.clone(),
                            visible_from,
                            canonical,
                        );
                    }
                }
                Match::Uncertain => result.uncertain = true,
                Match::No => {
                    if known_type_parent(modules, &target) {
                        changed |= aliases.insert_clean(scope, name.clone(), target.clone());
                    }
                }
            }
            let module = catalog.contains_module(&target, &scope.module);
            if module == Match::Uncertain {
                result.uncertain = true;
            }
            if !module.present() && !modules.contains(&target) {
                continue;
            }
            changed |= if type_match == Match::No {
                aliases.insert_clean(scope, name.clone(), target.clone())
            } else {
                aliases.insert(scope, name.clone(), target.clone())
            };
            if module_item {
                let visible_from =
                    super::visibility::scope(tokens, visibility_index, &scope.module);
                changed |= result.bind_module(index, name.clone(), target, visible_from);
            }
        }
    }
    changed
}

fn known_type_parent(modules: &ModuleIndex, target: &[String]) -> bool {
    target
        .split_last()
        .is_some_and(|(_, parent)| modules.contains(parent))
}
