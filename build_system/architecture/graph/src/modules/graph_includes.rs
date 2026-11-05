use super::declarations;
use super::files;
use super::includes;
use super::pending_states::{Pending, State};
use crate::modules::ModuleIndex;
use std::collections::BTreeSet;
use yolo_architecture_syntax::targeted_tokens::Token;

pub(super) fn extend(
    state: &State,
    tokens: &[Token],
    declarations: &[declarations::Declaration],
    available: &BTreeSet<String>,
    modules: &mut ModuleIndex,
    pending: &mut Pending,
) {
    for invocation in includes::parse(tokens)
        .into_iter()
        .filter(|invocation| invocation.enabled)
    {
        let context = declarations::context(declarations, invocation.index);
        if !context.enabled {
            continue;
        }
        let edge_product = state.product && !invocation.test_only && !context.test_only;
        if (invocation.uncertain || context.uncertain) && edge_product {
            modules.mark_uncertain(&state.file);
        }
        if !declarations::module_item(declarations, tokens, invocation.index) {
            if edge_product {
                modules.mark_uncertain(&state.file);
            }
            continue;
        }
        let Some(target) = invocation
            .literal
            .as_deref()
            .and_then(|literal| files::included(&state.file, literal))
            .filter(|target| available.contains(target))
        else {
            if edge_product {
                modules.mark_uncertain(&state.file);
            }
            continue;
        };
        if state.lineage.contains(&target) {
            if edge_product {
                modules.mark_uncertain(&state.file);
                modules.mark_uncertain(&target);
            }
            continue;
        }
        let mut path = state.path.clone();
        path.extend(declarations::inline_ancestors(tokens, invocation.index));
        let mut lineage = state.lineage.clone();
        lineage.insert(target.clone());
        let module_search_directory = files::containing_directory(&target);
        let Some(module_search_directory) = module_search_directory else {
            if edge_product {
                modules.mark_uncertain(&state.file);
            }
            continue;
        };
        if !pending.push(
            &state.file,
            State {
                file: target,
                path,
                lineage,
                crate_root: state.crate_root.clone(),
                module_search_directory: Some(module_search_directory),
                product: edge_product,
            },
            modules,
        ) {
            return;
        }
    }
}
