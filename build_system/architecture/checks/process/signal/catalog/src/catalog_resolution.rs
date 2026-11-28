use super::{Catalog, Match};
use std::collections::BTreeSet;

pub(super) fn resolve(catalog: &Catalog, path: &[String], scope: &[String], module: bool) -> Match {
    let mut pending = vec![path.to_vec()];
    let mut visited = BTreeSet::new();
    let mut retained = BTreeSet::new();
    let budget = catalog.resolution_budget(path);
    while let Some(candidate) = pending.pop() {
        match catalog.matches(&candidate, scope, module) {
            Match::Aggregate => {
                retained.insert(false);
            }
            Match::Canonical => {
                retained.insert(true);
            }
            Match::Uncertain => return Match::Uncertain,
            Match::No => {}
        }
        if !visited.insert(candidate.clone()) {
            continue;
        }
        if visited.len() > budget {
            return Match::Uncertain;
        }
        for alias in catalog.aliases_for(&candidate, scope, module) {
            if let Some(rewritten) = rewrite(&candidate, &alias.destination, &alias.target) {
                pending.push(rewritten);
            } else if module && alias.destination.starts_with(&candidate) {
                pending.push(alias.target.clone());
            }
        }
    }
    super::matched_kind(retained.into_iter())
}

pub(super) fn rewrite(path: &[String], from: &[String], to: &[String]) -> Option<Vec<String>> {
    path.starts_with(from).then(|| {
        to.iter()
            .cloned()
            .chain(path[from.len()..].iter().cloned())
            .collect()
    })
}

#[cfg(test)]
#[path = "../unit_tests/catalog_resolution_tests.rs"]
mod tests;
