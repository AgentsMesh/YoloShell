use super::Catalog;
use super::model::ModuleAlias;
use crate::VisibilityBoundaries;
use std::collections::BTreeSet;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct State {
    path: Vec<String>,
    boundary: Option<Vec<String>>,
}

impl Catalog {
    pub fn aggregate_boundaries(
        &self,
        path: &[String],
        scope: &[String],
    ) -> Option<VisibilityBoundaries> {
        resolve(self, path, scope)
    }
}

fn resolve(catalog: &Catalog, path: &[String], scope: &[String]) -> Option<VisibilityBoundaries> {
    let mut pending = vec![State {
        path: path.to_vec(),
        boundary: None,
    }];
    let mut visited = BTreeSet::new();
    let mut boundaries = BTreeSet::new();
    let budget = catalog.resolution_budget(path);
    while let Some(state) = pending.pop() {
        if !visited.insert(state.clone()) || visited.len() > budget {
            if visited.len() > budget {
                return None;
            }
            continue;
        }
        collect_matches(catalog, &state, scope, &mut boundaries)?;
        for alias in catalog.aliases_for(&state.path, scope, false) {
            let Some(path) =
                super::resolution::rewrite(&state.path, &alias.destination, &alias.target)
            else {
                continue;
            };
            let boundary = alias_boundary(alias, state.boundary.as_deref(), scope)?;
            pending.push(State { path, boundary });
        }
    }
    (!boundaries.is_empty()).then_some(boundaries)
}

fn collect_matches(
    catalog: &Catalog,
    state: &State,
    scope: &[String],
    boundaries: &mut VisibilityBoundaries,
) -> Option<()> {
    if let Some(entries) = catalog.exact_scopes.get(&state.path) {
        for entry in entries
            .iter()
            .filter(|entry| entry.declaration && scope.starts_with(&entry.visible_from))
        {
            if entry.canonical {
                return None;
            }
            boundaries.insert(narrowest(
                state.boundary.as_deref(),
                &entry.visible_from,
                scope,
            )?);
        }
    }
    Some(())
}

fn alias_boundary(
    alias: &ModuleAlias,
    current: Option<&[String]>,
    scope: &[String],
) -> Option<Option<Vec<String>>> {
    let parent = alias.destination.split_last()?.1;
    if alias.visible_from == parent {
        return Some(current.map(<[String]>::to_vec));
    }
    Some(Some(narrowest(current, &alias.visible_from, scope)?))
}

fn narrowest(
    current: Option<&[String]>,
    candidate: &[String],
    scope: &[String],
) -> Option<Vec<String>> {
    if !scope.starts_with(candidate) {
        return None;
    }
    let Some(current) = current else {
        return Some(candidate.to_vec());
    };
    if current.starts_with(candidate) {
        Some(current.to_vec())
    } else if candidate.starts_with(current) {
        Some(candidate.to_vec())
    } else {
        None
    }
}

#[cfg(test)]
#[path = "../../unit_tests/catalog_boundary_adversarial_tests.rs"]
mod adversarial_tests;
