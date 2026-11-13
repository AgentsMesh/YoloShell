#[path = "catalog/boundaries.rs"]
mod boundaries;
#[cfg(test)]
#[path = "../unit_tests/catalog_boundary_tests.rs"]
mod boundary_tests;
#[path = "catalog/children.rs"]
mod children;
#[path = "catalog/indexes.rs"]
mod indexes;
#[path = "catalog/model.rs"]
mod model;

#[path = "catalog_resolution.rs"]
mod resolution;

pub use model::Catalog;
pub use model::Match;
use model::{ModuleAlias, RetainedEntry, matched_kind};

const MINIMUM_RESOLUTION_FACTOR: usize = 1;
const MAXIMUM_PATH_RESOLUTION_STATES: usize = 4_096;

pub struct Children<T> {
    pub values: Vec<T>,
    pub uncertain: bool,
}

impl Catalog {
    pub fn merge(&mut self, additions: Self) -> bool {
        let mut changed = false;
        for entry in additions.entries {
            let Some((name, module)) = entry.path.split_last() else {
                continue;
            };
            changed |= self.insert(module.to_vec(), name, &entry.visible_from, entry.canonical);
        }
        for alias in additions.aliases {
            let inserted = if alias.module {
                self.alias_module(&alias.destination, &alias.target, &alias.visible_from)
            } else {
                self.alias_type(
                    &alias.destination,
                    &alias.target,
                    &alias.visible_from,
                    alias.canonical,
                )
            };
            changed |= inserted;
        }
        changed
    }

    pub fn insert(
        &mut self,
        mut module: Vec<String>,
        name: &str,
        visible_from: &[String],
        canonical: bool,
    ) -> bool {
        module.push(name.to_owned());
        let entry = RetainedEntry {
            path: module,
            visible_from: visible_from.to_vec(),
            canonical,
            declaration: true,
        };
        let inserted = self.entries.insert(entry.clone());
        if inserted {
            self.direct_types
                .entry(name.to_owned())
                .or_default()
                .insert(entry.clone());
            self.index_entry(&entry);
        }
        inserted
    }

    pub fn alias_module(
        &mut self,
        destination: &[String],
        target: &[String],
        visible_from: &[String],
    ) -> bool {
        self.insert_alias(ModuleAlias {
            destination: destination.to_vec(),
            target: target.to_vec(),
            visible_from: visible_from.to_vec(),
            module: true,
            canonical: false,
        })
    }

    pub fn alias_type(
        &mut self,
        destination: &[String],
        target: &[String],
        visible_from: &[String],
        canonical: bool,
    ) -> bool {
        let alias = ModuleAlias {
            destination: destination.to_vec(),
            target: target.to_vec(),
            visible_from: visible_from.to_vec(),
            module: false,
            canonical,
        };
        let inserted = self.insert_alias(alias);
        if inserted && let Some(name) = destination.last().cloned() {
            self.direct_types
                .entry(name)
                .or_default()
                .insert(RetainedEntry {
                    path: destination.to_vec(),
                    visible_from: visible_from.to_vec(),
                    canonical,
                    declaration: false,
                });
        }
        inserted
    }

    pub fn contains(&self, path: &[String], scope: &[String]) -> Match {
        self.resolve(path, scope, false)
    }

    pub fn may_contain_type(&self, name: &str) -> bool {
        self.direct_types.contains_key(name)
    }

    pub fn contains_direct(&self, path: &[String], scope: &[String]) -> Match {
        let Some(name) = path.last() else {
            return Match::No;
        };
        let kinds = self
            .direct_types
            .get(name)
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                (entry.path == path && entry_accessible(entry, scope)).then_some(entry.canonical)
            });
        matched_kind(kinds)
    }

    pub fn contains_module(&self, path: &[String], scope: &[String]) -> Match {
        self.resolve(path, scope, true)
    }

    fn resolve(&self, path: &[String], scope: &[String], module: bool) -> Match {
        if let Some(result) = self.resolution_cache.borrow().get(path, scope, module) {
            return result;
        }
        let result = resolution::resolve(self, path, scope, module);
        self.resolution_cache
            .borrow_mut()
            .insert(path, scope, module, result);
        result
    }

    fn matches(&self, candidate: &[String], scope: &[String], module: bool) -> Match {
        if module {
            return self
                .module_scopes
                .get(candidate)
                .is_some_and(|roots| roots.iter().any(|root| scope.starts_with(root)))
                .then_some(Match::Aggregate)
                .unwrap_or(Match::No);
        }
        let kinds = self
            .exact_scopes
            .get(candidate)
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                scope
                    .starts_with(&entry.visible_from)
                    .then_some(entry.canonical)
            });
        matched_kind(kinds)
    }

    fn resolution_budget(&self, path: &[String]) -> usize {
        self.aliases
            .len()
            .max(MINIMUM_RESOLUTION_FACTOR)
            .saturating_mul(path.len().max(MINIMUM_RESOLUTION_FACTOR))
            .min(MAXIMUM_PATH_RESOLUTION_STATES)
    }
}

pub type VisibilityBoundaries = std::collections::BTreeSet<Vec<String>>;

fn alias_accessible(alias: &ModuleAlias, scope: &[String]) -> bool {
    scope.starts_with(&alias.visible_from)
}

fn entry_accessible(entry: &RetainedEntry, scope: &[String]) -> bool {
    scope.starts_with(&entry.visible_from)
}
