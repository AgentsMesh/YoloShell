#[path = "aliases/clean.rs"]
mod clean;
#[path = "aliases/declarations.rs"]
mod declarations;

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use yolo_architecture_build_graph::path_syntax::{
    PARENT_COMPONENT, ROOT_COMPONENT, SELF_COMPONENT, path_candidates,
};

const FIRST_COMPONENT_OFFSET: usize = 1;

#[derive(Clone)]
pub(super) struct Scope {
    pub module: Vec<String>,
    blocks: Vec<usize>,
    position: usize,
}

#[derive(Clone, Default)]
pub(super) struct Aliases {
    bindings: BTreeMap<(Vec<String>, Vec<usize>, String), BTreeSet<Vec<String>>>,
    clean_bindings: BTreeSet<(Vec<String>, Vec<usize>, String)>,
    declarations: BTreeSet<(Vec<String>, Vec<usize>, String)>,
    declaration_sites: BTreeMap<(Vec<String>, Vec<usize>, String), BTreeSet<usize>>,
    module_declaration_sites: BTreeMap<(Vec<String>, String), BTreeSet<(Vec<String>, usize)>>,
    shadows: BTreeSet<(Vec<String>, Vec<usize>, String)>,
    ranged_shadows: BTreeSet<(Vec<String>, String, usize, usize)>,
}

impl Scope {
    pub fn at(module: Vec<String>, blocks: &[usize], index: usize) -> Self {
        Self {
            module,
            blocks: blocks.to_vec(),
            position: index,
        }
    }
}

impl Aliases {
    pub fn insert(&mut self, scope: &Scope, name: String, target: Vec<String>) -> bool {
        self.bindings
            .entry((scope.module.clone(), scope.blocks.clone(), name))
            .or_default()
            .insert(target)
    }

    pub fn shadow(&mut self, scope: &Scope, name: String) -> bool {
        self.shadows
            .insert((scope.module.clone(), scope.blocks.clone(), name))
    }

    pub fn shadow_range(&mut self, module: Vec<String>, name: String, range: Range<usize>) -> bool {
        self.ranged_shadows
            .insert((module, name, range.start, range.end))
    }

    pub fn candidates(&self, raw: &[String], scopes: &[Scope]) -> Vec<Vec<String>> {
        let mut result = BTreeSet::new();
        for scope in scopes {
            match self.local_targets(raw, scope) {
                Some(targets) => result.extend(targets),
                None => result.extend(path_candidates(raw, &scope.module)),
            }
        }
        result.into_iter().collect()
    }

    fn local_targets(&self, raw: &[String], scope: &Scope) -> Option<Vec<Vec<String>>> {
        if explicitly_rooted(raw) {
            return None;
        }
        let (module, name, suffix) = binding_site(raw, &scope.module)?;
        if module != scope.module {
            return None;
        }
        if self
            .ranged_shadows
            .iter()
            .any(|(bound_module, bound, start, end)| {
                bound_module == &module && bound == name && (*start..*end).contains(&scope.position)
            })
        {
            return Some(Vec::new());
        }
        let depth = self.binding_depth(&module, name, &scope.blocks)?;
        if bound_at_depth(&self.shadows, &module, name, &scope.blocks, depth) {
            return Some(Vec::new());
        }
        if bound_at_depth(&self.declarations, &module, name, &scope.blocks, depth) {
            let target = module
                .iter()
                .cloned()
                .chain(std::iter::once(name.to_owned()))
                .chain(raw[suffix..].iter().cloned())
                .collect();
            return Some(vec![target]);
        }
        Some(
            self.bindings
                .iter()
                .filter(|((bound_module, blocks, bound), _)| {
                    bound_module == &module
                        && bound == name
                        && blocks.len() == depth
                        && scope.blocks.starts_with(blocks)
                })
                .flat_map(|(_, targets)| targets)
                .map(|target| {
                    target
                        .iter()
                        .cloned()
                        .chain(raw[suffix..].iter().cloned())
                        .collect()
                })
                .collect(),
        )
    }

    fn binding_depth(&self, module: &[String], name: &str, blocks: &[usize]) -> Option<usize> {
        self.bindings
            .keys()
            .map(|(scope, blocks, name)| (scope, blocks, name))
            .chain(
                self.declarations
                    .iter()
                    .map(|(scope, blocks, name)| (scope, blocks, name)),
            )
            .chain(
                self.shadows
                    .iter()
                    .map(|(scope, blocks, name)| (scope, blocks, name)),
            )
            .filter(|(bound_module, bound_blocks, bound)| {
                bound_module.as_slice() == module
                    && bound == &name
                    && blocks.starts_with(bound_blocks)
            })
            .map(|(_, bound_blocks, _)| bound_blocks.len())
            .max()
    }
}

fn explicitly_rooted(raw: &[String]) -> bool {
    raw.first().is_some_and(|component| {
        matches!(
            component.as_str(),
            PARENT_COMPONENT | ROOT_COMPONENT | SELF_COMPONENT
        )
    })
}

fn bound_at_depth(
    bindings: &BTreeSet<(Vec<String>, Vec<usize>, String)>,
    module: &[String],
    name: &str,
    blocks: &[usize],
    depth: usize,
) -> bool {
    bindings.iter().any(|(bound_module, bound_blocks, bound)| {
        bound_module == module
            && bound == name
            && bound_blocks.len() == depth
            && blocks.starts_with(bound_blocks)
    })
}

fn binding_site<'a>(raw: &'a [String], module: &[String]) -> Option<(Vec<String>, &'a str, usize)> {
    let first = raw.first()?.as_str();
    if first == ROOT_COMPONENT {
        return Some((vec![module.first()?.clone()], raw.get(1)?, 2));
    }
    if first == SELF_COMPONENT {
        return Some((module.to_vec(), raw.get(1)?, 2));
    }
    if first == PARENT_COMPONENT {
        let parents = raw
            .iter()
            .take_while(|part| part.as_str() == PARENT_COMPONENT)
            .count();
        let keep = module
            .len()
            .saturating_sub(parents)
            .max(FIRST_COMPONENT_OFFSET);
        return Some((module[..keep].to_vec(), raw.get(parents)?, parents + 1));
    }
    Some((module.to_vec(), first, FIRST_COMPONENT_OFFSET))
}

#[cfg(test)]
#[path = "../../unit_tests/alias_shadow_boundary_tests.rs"]
mod boundary_tests;
