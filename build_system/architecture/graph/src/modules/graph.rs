#[path = "graph_budget.rs"]
mod budget;
#[path = "graph_includes.rs"]
mod include_edges;
#[path = "graph_pending.rs"]
mod pending_states;
#[path = "graph_seeds.rs"]
mod seed_states;

use self::pending_states::{Pending, State};
use super::{ModuleIndex, declarations, files, includes};
use crate::crates::Catalog as CrateCatalog;
use std::collections::{BTreeMap, BTreeSet};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_syntax::rust_source::RustSource;

pub(super) fn discover(
    sources: &[(&SourceFile, RustSource)],
    crates: &CrateCatalog,
) -> ModuleIndex {
    let parsed = sources
        .iter()
        .map(|(file, source)| (file.relative.as_str(), source))
        .collect::<BTreeMap<_, _>>();
    let available = parsed.keys().map(|path| (*path).to_owned()).collect();
    let mut modules = ModuleIndex::default();
    modules.set_declared_crate_roots(!crates.roots.is_empty());
    for (relative, source) in &parsed {
        modules.index_syntax(relative, &source.tokens);
    }
    let seeds = seed_states::seeds(sources, crates, &available, &mut modules);
    let budget = graph_budget(sources, seeds.len());
    let mut pending = Pending::new(seeds, budget, &mut modules);
    let mut visited = BTreeSet::new();
    while !pending.exhausted() {
        let Some(state) = pending.pop_front() else {
            break;
        };
        if !visited.insert((state.file.clone(), state.path.clone(), state.product)) {
            continue;
        }
        if state.product {
            modules.mark_product(&state.file);
        }
        modules.insert(&state.file, state.path.clone());
        let Some(source) = parsed.get(state.file.as_str()) else {
            continue;
        };
        extend(
            &state,
            &source.tokens,
            &available,
            &mut modules,
            &mut pending,
        );
    }
    modules
}

fn graph_budget(sources: &[(&SourceFile, RustSource)], roots: usize) -> usize {
    let edges = sources
        .iter()
        .map(|(_, source)| {
            declarations::parse(&source.tokens).len() + includes::parse(&source.tokens).len()
        })
        .sum::<usize>();
    budget::states(roots, sources.len(), edges)
}

fn extend(
    state: &State,
    tokens: &[yolo_architecture_syntax::targeted_tokens::Token],
    available: &BTreeSet<String>,
    modules: &mut ModuleIndex,
    pending: &mut Pending,
) {
    let items = declarations::parse(tokens);
    for declaration in items.iter().filter(|item| item.enabled) {
        let context = declarations::context(&items, declaration.index);
        if !context.enabled {
            continue;
        }
        let edge_product = state.product && !declaration.test_only && !context.test_only;
        if (declaration.uncertain || context.uncertain) && edge_product {
            modules.mark_uncertain(&state.file);
        }
        let ancestors = declarations::ancestors(&items, declaration.index);
        let mut path = state.path.clone();
        path.extend(
            ancestors
                .iter()
                .copied()
                .chain(std::iter::once(declaration.name.as_str()))
                .map(str::to_owned),
        );
        if declaration.body.is_some() {
            modules.remember(&path);
            continue;
        }
        if !declarations::module_item(&items, tokens, declaration.index) {
            if edge_product {
                modules.mark_uncertain(&state.file);
            }
            continue;
        }
        let Some(target) = files::target(
            &state.file,
            state.module_search_directory.as_deref(),
            declaration,
            &ancestors,
            available,
            state.crate_root.as_deref(),
        ) else {
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
        let mut lineage = state.lineage.clone();
        lineage.insert(target.clone());
        let module_search_directory = declaration
            .path
            .as_ref()
            .and_then(|_| files::containing_directory(&target));
        if !pending.push(
            &state.file,
            State {
                file: target,
                path,
                lineage,
                crate_root: state.crate_root.clone(),
                module_search_directory,
                product: edge_product,
            },
            modules,
        ) {
            return;
        }
    }
    if state.product && declarations::unparsed(tokens) {
        modules.mark_uncertain(&state.file);
    }
    if !pending.exhausted() {
        include_edges::extend(state, tokens, &items, available, modules, pending);
    }
}

#[cfg(test)]
#[path = "../../unit_tests/graph_transition_tests.rs"]
mod tests;
