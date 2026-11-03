//! Stable topological order for recipe agents. Dependencies name earlier agents;
//! the file order only breaks ties.

use super::RecipeDocument;
use crate::constants::*;
use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};

/// Return agent indices in dependency order. Unknown, self, duplicate or
/// cyclic dependencies are rejected.
pub(crate) fn compute(document: &RecipeDocument) -> Result<Vec<usize>> {
    let mut names = BTreeMap::new();
    for (index, agent) in document.agents.iter().enumerate() {
        if let Some(name) = agent.name.as_deref() {
            names.insert(name.to_ascii_lowercase(), index);
        }
    }
    let mut pending = vec![0_usize; document.agents.len()];
    let mut dependents = vec![Vec::new(); document.agents.len()];
    for (index, agent) in document.agents.iter().enumerate() {
        let mut seen = BTreeSet::new();
        for dependency in &agent.after {
            let key = dependency.to_ascii_lowercase();
            if dependency.trim().is_empty() || !seen.insert(key.clone()) {
                bail!(INVALID_RECIPE_DAG);
            }
            let Some(parent) = names.get(&key).copied().filter(|parent| *parent != index) else {
                bail!(INVALID_RECIPE_DAG);
            };
            pending[index] += 1;
            dependents[parent].push(index);
        }
    }
    let mut ready = (0..document.agents.len())
        .filter(|index| pending[*index] == 0)
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(document.agents.len());
    while let Some(index) = ready.pop_first() {
        order.push(index);
        for dependent in &dependents[index] {
            pending[*dependent] -= 1;
            if pending[*dependent] == 0 {
                ready.insert(*dependent);
            }
        }
    }
    if order.len() != document.agents.len() {
        bail!(INVALID_RECIPE_DAG);
    }
    Ok(order)
}
