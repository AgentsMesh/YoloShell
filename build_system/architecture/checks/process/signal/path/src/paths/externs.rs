use super::aliases::Aliases;
use super::{Resolution, scopes, visibility};
use yolo_architecture_build_graph::modules::ModuleIndex;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::Token;

const EXTERN_KEYWORD: &str = "extern";
const CRATE_KEYWORD: &str = "crate";
const SELF_KEYWORD: &str = "self";
const ALIAS_KEYWORD: &str = "as";
const EXTERN_ALIAS_TOKEN_COUNT: usize = 5;
const EXTERN_OFFSET: usize = 0;
const CRATE_OFFSET: usize = 1;
const SELF_OFFSET: usize = 2;
const AS_OFFSET: usize = 3;
const ALIAS_OFFSET: usize = 4;

pub(super) fn scan(
    relative: &str,
    tokens: &[Token],
    modules: &ModuleIndex,
    aliases: &mut Aliases,
    result: &mut Resolution,
) {
    for (index, window) in tokens.windows(EXTERN_ALIAS_TOKEN_COUNT).enumerate() {
        if !window[EXTERN_OFFSET].identifier(EXTERN_KEYWORD)
            || !window[CRATE_OFFSET].identifier(CRATE_KEYWORD)
            || !window[SELF_OFFSET].identifier(SELF_KEYWORD)
            || !window[AS_OFFSET].identifier(ALIAS_KEYWORD)
        {
            continue;
        }
        let Some(name) = identifier(&window[ALIAS_OFFSET]).map(str::to_owned) else {
            continue;
        };
        for scope in scopes::at(relative, index, modules) {
            let Some(root) = scope.module.first().cloned() else {
                continue;
            };
            let target = vec![root];
            aliases.insert(&scope, name.clone(), target.clone());
            if modules.module_item(relative, index) {
                let visible_from = visibility::scope(tokens, index, &scope.module);
                result.bind_module(index, name.clone(), target, visible_from);
            }
        }
    }
}

#[cfg(test)]
#[path = "../../unit_tests/extern_alias_boundary_tests.rs"]
mod boundary_tests;
