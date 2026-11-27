use super::super::aliases::{Aliases, Scope};
use super::super::{Catalog, Resolution};
use yolo_architecture_process_signal_catalog::Match;
use yolo_architecture_build_graph::modules::ModuleIndex;
use yolo_architecture_syntax::targeted_tokens::Token;

#[allow(clippy::too_many_arguments)]
pub(super) fn import(
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
    let mut changed = false;
    for scope in scopes {
        for module in aliases.candidates(raw, std::slice::from_ref(scope)) {
            let matched = catalog.contains_module(&module, &scope.module);
            if matched == Match::Uncertain {
                result.uncertain = true;
            }
            if !matched.present() && !modules.contains(&module) {
                continue;
            }
            let types = catalog.type_children(&module, &scope.module);
            result.uncertain |= types.uncertain;
            for (name, canonical, boundaries) in types.values {
                changed |= result.bind_type(index, name.clone(), canonical, boundaries.as_ref());
                if module_item {
                    let mut target = module.clone();
                    target.push(name.clone());
                    let visible_from =
                        super::super::visibility::scope(tokens, visibility_index, &scope.module);
                    changed |= result.bind_type_alias(index, name, target, visible_from, canonical);
                }
            }
            let children = catalog.module_children(&module, &scope.module);
            result.uncertain |= children.uncertain;
            for (name, target) in children.values {
                changed |= aliases.insert(scope, name.clone(), target.clone());
                if module_item {
                    let visible_from =
                        super::super::visibility::scope(tokens, visibility_index, &scope.module);
                    changed |= result.bind_module(index, name, target, visible_from);
                }
            }
        }
    }
    changed
}

#[cfg(test)]
#[path = "../../../unit_tests/glob_import_boundaries.rs"]
mod tests;
