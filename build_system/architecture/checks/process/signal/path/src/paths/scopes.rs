use super::aliases::{Aliases, Scope};
use std::collections::BTreeSet;
use yolo_architecture_build_graph::modules::ModuleIndex;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const BLOCK_CLOSE: &str = "}";
const BLOCK_OPEN: &str = "{";
const ITEM_END: &str = ";";
const NEXT_TOKEN_OFFSET: usize = 1;
const ITEM_NAME_OFFSET: usize = 1;
const AFTER_ITEM_NAME_OFFSET: usize = 2;
const DECLARATION_NAME_OFFSET: usize = 1;
const SHADOWING_ITEMS: &[&str] = &["enum", "mod", "struct", "trait", "type", "union"];
const GENERIC_ITEMS: &[&str] = &["enum", "fn", "impl", "struct", "trait", "type", "union"];
const GENERIC_OPEN: &str = "<";
const GENERIC_CLOSE: &str = ">";
const GENERIC_DOUBLE_CLOSE: &str = ">>";
const ITEM_SEPARATOR: &str = ",";
const LIFETIME_PREFIX: &str = "'";
const CONST_KEYWORD: &str = "const";
const IMPLEMENTATION_KEYWORD: &str = "impl";
const MODULE_KEYWORD: &str = "mod";

pub(super) fn at(relative: &str, index: usize, modules: &ModuleIndex) -> Vec<Scope> {
    modules
        .paths_at(relative, index)
        .into_iter()
        .map(|module| Scope::at(module, modules.blocks_at(relative, index), index))
        .collect()
}

pub(super) fn install_shadows(
    relative: &str,
    tokens: &[Token],
    modules: &ModuleIndex,
    aliases: &mut Aliases,
) {
    for (index, token) in tokens.iter().enumerate() {
        if !SHADOWING_ITEMS
            .iter()
            .any(|keyword| token.identifier(keyword))
        {
            continue;
        }
        let Some(name) = tokens
            .get(index + DECLARATION_NAME_OFFSET)
            .and_then(identifier)
            .map(str::to_owned)
        else {
            continue;
        };
        for scope in at(relative, index, modules) {
            if token.identifier(MODULE_KEYWORD) {
                if modules.module_item(relative, index) {
                    let mut target = scope.module.clone();
                    target.push(name.clone());
                    aliases.insert(&scope, name.clone(), target);
                } else {
                    aliases.shadow(&scope, name.clone());
                }
            } else {
                let visible_from = modules
                    .module_item(relative, index)
                    .then(|| super::visibility::scope(tokens, index, &scope.module));
                aliases.declare(&scope, name.clone(), visible_from);
            }
        }
    }
    install_generic_shadows(relative, tokens, modules, aliases);
}

fn install_generic_shadows(
    relative: &str,
    tokens: &[Token],
    modules: &ModuleIndex,
    aliases: &mut Aliases,
) {
    for (index, token) in tokens.iter().enumerate() {
        if !GENERIC_ITEMS
            .iter()
            .any(|keyword| token.identifier(keyword))
        {
            continue;
        }
        let Some(boundary) = tokens[index..]
            .iter()
            .position(|token| token.symbol(BLOCK_OPEN) || token.symbol(ITEM_END))
            .map(|offset| index + offset)
        else {
            continue;
        };
        let Some(open) = generic_open(tokens, index, boundary) else {
            continue;
        };
        let names = generic_names(tokens, open, boundary);
        let close = if tokens[boundary].symbol(BLOCK_OPEN) {
            closing(tokens, boundary, BLOCK_OPEN, BLOCK_CLOSE).unwrap_or(boundary)
        } else {
            boundary
        };
        for module in modules.paths_at(relative, index) {
            for name in &names {
                aliases.shadow_range(
                    module.clone(),
                    name.clone(),
                    open..close + NEXT_TOKEN_OFFSET,
                );
            }
        }
    }
}

fn generic_open(tokens: &[Token], item: usize, end: usize) -> Option<usize> {
    let offset = if tokens[item].identifier(IMPLEMENTATION_KEYWORD) {
        ITEM_NAME_OFFSET
    } else {
        AFTER_ITEM_NAME_OFFSET
    };
    let open = item + offset;
    (open < end && tokens[open].symbol(GENERIC_OPEN)).then_some(open)
}

fn generic_names(tokens: &[Token], open: usize, end: usize) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut depth = 0_usize;
    let mut parameter = false;
    let mut value_namespace = false;
    for token in &tokens[open..end] {
        if token.symbol(GENERIC_OPEN) {
            depth += 1;
            parameter |= depth == 1;
        } else if token.symbol(GENERIC_CLOSE) {
            depth = depth.saturating_sub(1);
        } else if token.symbol(GENERIC_DOUBLE_CLOSE) {
            depth = depth.saturating_sub(2);
        } else if depth == 1 && token.symbol(ITEM_SEPARATOR) {
            parameter = true;
            value_namespace = false;
        } else if depth == 1
            && parameter
            && (token.symbol(LIFETIME_PREFIX) || token.identifier(CONST_KEYWORD))
        {
            value_namespace = true;
        } else if depth == 1
            && parameter
            && let Some(name) = identifier(token)
        {
            if !value_namespace {
                names.insert(name.to_owned());
            }
            parameter = false;
            value_namespace = false;
        }
    }
    names
}
