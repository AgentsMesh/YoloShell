use super::{Catalog, Children, Match};
use crate::VisibilityBoundaries;
use std::collections::BTreeSet;

impl Catalog {
    pub fn type_children(
        &self,
        module: &[String],
        scope: &[String],
    ) -> Children<(String, bool, Option<VisibilityBoundaries>)> {
        let candidates = self.module_candidates(module, scope);
        let names = candidates
            .values
            .iter()
            .filter_map(|candidate| self.type_child_names.get(candidate))
            .flatten()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut result = Children {
            values: Vec::new(),
            uncertain: candidates.uncertain,
        };
        for name in names {
            let mut candidate = module.to_vec();
            candidate.push(name.clone());
            match self.contains(&candidate, scope) {
                Match::Aggregate => match self.aggregate_boundaries(&candidate, scope) {
                    Some(boundaries) => result.values.push((name, false, Some(boundaries))),
                    None => result.uncertain = true,
                },
                Match::Canonical => result.values.push((name, true, None)),
                Match::Uncertain => result.uncertain = true,
                Match::No => {}
            }
        }
        result
    }

    pub fn module_children(
        &self,
        module: &[String],
        scope: &[String],
    ) -> Children<(String, Vec<String>)> {
        let candidates = self.module_candidates(module, scope);
        let names = candidates
            .values
            .iter()
            .filter_map(|candidate| self.module_child_names.get(candidate))
            .flatten()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut result = Children {
            values: Vec::new(),
            uncertain: candidates.uncertain,
        };
        for name in names {
            let mut target = module.to_vec();
            target.push(name.clone());
            match self.contains_module(&target, scope) {
                Match::Aggregate | Match::Canonical => result.values.push((name, target)),
                Match::Uncertain => result.uncertain = true,
                Match::No => {}
            }
        }
        result
    }

    fn module_candidates(&self, module: &[String], scope: &[String]) -> Children<Vec<String>> {
        let mut pending = vec![module.to_vec()];
        let mut visited = BTreeSet::new();
        let budget = self.resolution_budget(module);
        while let Some(candidate) = pending.pop() {
            if !visited.insert(candidate.clone()) {
                continue;
            }
            if visited.len() > budget {
                return Children {
                    values: visited.into_iter().collect(),
                    uncertain: true,
                };
            }
            for alias in self.aliases_for(&candidate, scope, true) {
                if let Some(rewritten) =
                    super::resolution::rewrite(&candidate, &alias.destination, &alias.target)
                {
                    pending.push(rewritten);
                }
            }
        }
        Children {
            values: visited.into_iter().collect(),
            uncertain: false,
        }
    }
}
