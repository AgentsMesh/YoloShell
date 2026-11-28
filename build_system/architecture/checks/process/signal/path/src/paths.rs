#[path = "paths/aliases.rs"]
mod aliases;
#[path = "paths/externs.rs"]
mod externs;
#[path = "paths/facts.rs"]
mod facts;
#[path = "paths/imports.rs"]
mod imports;
#[path = "paths/result.rs"]
mod result;
#[path = "paths/scopes.rs"]
pub(crate) mod scopes;
#[path = "paths/visibility.rs"]
pub mod visibility;

pub use facts::{ResolutionFacts, prepare};
pub use yolo_architecture_process_signal_catalog::{Catalog, VisibilityBoundaries};

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use yolo_architecture_process_signal_catalog::Match;
use yolo_architecture_build_graph::modules::ModuleIndex;
use yolo_architecture_build_graph::path_syntax::{backward_path, statement_end};
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::Token;

const USE_KEYWORD: &str = "use";
const MINIMUM_RESOLUTION_PASSES: usize = 1;
const MAXIMUM_IMPORT_RESOLUTION_PASSES: usize = 4_096;
const MINIMUM_QUALIFIED_PATH_COMPONENTS: usize = 2;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModuleBinding {
    pub index: usize,
    pub name: String,
    pub target: Vec<String>,
    pub visible_from: Vec<String>,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct TypeBinding {
    pub index: usize,
    pub name: String,
    pub target: Vec<String>,
    pub visible_from: Vec<String>,
    pub canonical: bool,
}

#[derive(Clone, Default)]
pub struct Resolution {
    pub names: BTreeSet<String>,
    pub indices: BTreeSet<usize>,
    pub clean_indices: BTreeSet<usize>,
    pub declaration_indices: BTreeMap<usize, BTreeSet<usize>>,
    pub canonical_names: BTreeSet<String>,
    pub canonical_indices: BTreeSet<usize>,
    pub aggregate_names: BTreeMap<String, VisibilityBoundaries>,
    pub aggregate_indices: BTreeMap<usize, VisibilityBoundaries>,
    pub module_bindings: BTreeSet<ModuleBinding>,
    pub type_bindings: BTreeSet<TypeBinding>,
    pub uncertain: bool,
}

pub fn resolve_prepared(
    relative: &str,
    tokens: &[Token],
    catalog: &Catalog,
    modules: &ModuleIndex,
    facts: &ResolutionFacts,
) -> Resolution {
    let imports = &facts.imports;
    let mut aliases = facts.aliases.clone();
    let mut result = facts.base.clone();
    let mut remaining = imports
        .len()
        .max(MINIMUM_RESOLUTION_PASSES)
        .min(MAXIMUM_IMPORT_RESOLUTION_PASSES);
    loop {
        let mut changed = false;
        for import in imports {
            let scopes = facts.scopes_at(relative, import.start, modules);
            changed |= imports::scan(
                tokens,
                relative,
                import.clone(),
                scopes.as_ref(),
                catalog,
                modules,
                &mut aliases,
                &mut result,
            );
        }
        if !changed {
            break;
        }
        if remaining == 0 {
            result.uncertain = true;
            break;
        }
        remaining = remaining.saturating_sub(MINIMUM_RESOLUTION_PASSES);
    }
    for (index, token) in tokens.iter().enumerate() {
        if imports.iter().any(|range| range.contains(&index)) || identifier(token).is_none() {
            continue;
        }
        let raw = backward_path(tokens, index);
        let direct = raw.len() < MINIMUM_QUALIFIED_PATH_COMPONENTS;
        let scopes = facts.scopes_at(relative, index, modules);
        let locally_bound = scopes
            .iter()
            .any(|scope| aliases.locally_clean(&raw, scope));
        let locally_declared = scopes
            .iter()
            .any(|scope| !aliases.local_declarations(&raw, scope).is_empty());
        if direct
            && !locally_bound
            && !locally_declared
            && !raw
                .first()
                .is_some_and(|name| catalog.may_contain_type(name) || result.names.contains(name))
        {
            continue;
        }
        let mut clean = !scopes.is_empty();
        for scope in scopes.iter() {
            for declaration in aliases.local_declarations(&raw, scope) {
                result.bind_declaration(index, declaration);
            }
            let bound = aliases.locally_clean(&raw, scope);
            let candidates = aliases.candidates(&raw, std::slice::from_ref(scope));
            let mut retained = false;
            let mut uncertain = false;
            for path in candidates {
                let matched = if direct {
                    catalog.contains_direct(&path, &scope.module)
                } else {
                    catalog.contains(&path, &scope.module)
                };
                match matched {
                    Match::Aggregate => match catalog.aggregate_boundaries(&path, &scope.module) {
                        Some(boundaries) => {
                            result.bind_aggregate(index, &boundaries);
                            retained = true;
                        }
                        None => uncertain = true,
                    },
                    Match::Canonical => {
                        result.bind_canonical(index);
                        retained = true;
                    }
                    Match::Uncertain => uncertain = true,
                    Match::No => {}
                }
            }
            result.uncertain |= uncertain;
            clean &= bound && !retained && !uncertain;
        }
        if clean {
            result.bind_clean(index);
        }
    }
    result
}

fn import_ranges(tokens: &[Token]) -> Vec<Range<usize>> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.identifier(USE_KEYWORD))
        .map(|(index, _)| index..statement_end(tokens, index))
        .collect()
}
