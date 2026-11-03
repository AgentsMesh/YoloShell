use super::{
    Aliases, FIRST_COMPONENT_OFFSET, Scope, binding_site, bound_at_depth, explicitly_rooted,
};
use std::collections::BTreeSet;
use yolo_architecture_build_graph::path_syntax::{
    PARENT_COMPONENT, ROOT_COMPONENT, SELF_COMPONENT, path_candidates,
};

impl Aliases {
    pub fn declare(
        &mut self,
        scope: &Scope,
        name: String,
        visible_from: Option<Vec<String>>,
    ) -> bool {
        let key = (scope.module.clone(), scope.blocks.clone(), name);
        let changed = self.declarations.insert(key.clone());
        let module_changed = visible_from.is_some_and(|boundary| {
            self.module_declaration_sites
                .entry((scope.module.clone(), key.2.clone()))
                .or_default()
                .insert((boundary, scope.position))
        });
        self.declaration_sites
            .entry(key)
            .or_default()
            .insert(scope.position)
            || changed
            || module_changed
    }

    pub fn local_declarations(&self, raw: &[String], scope: &Scope) -> BTreeSet<usize> {
        if explicitly_rooted(raw) {
            return self.rooted_declarations(raw, scope);
        }
        let Some((module, name, _)) = binding_site(raw, &scope.module) else {
            return BTreeSet::new();
        };
        if module != scope.module || self.ranged_shadow(&module, name, scope.position) {
            return BTreeSet::new();
        }
        let Some(depth) = self.binding_depth(&module, name, &scope.blocks) else {
            return BTreeSet::new();
        };
        if bound_at_depth(&self.shadows, &module, name, &scope.blocks, depth)
            || !bound_at_depth(&self.declarations, &module, name, &scope.blocks, depth)
        {
            return BTreeSet::new();
        }
        self.declaration_sites
            .iter()
            .filter(|((bound_module, blocks, bound), _)| {
                bound_module == &module
                    && bound == name
                    && blocks.len() == depth
                    && scope.blocks.starts_with(blocks)
            })
            .flat_map(|(_, sites)| sites.iter().copied())
            .collect()
    }

    fn rooted_declarations(&self, raw: &[String], scope: &Scope) -> BTreeSet<usize> {
        if !rooted_target_present(raw) {
            return BTreeSet::new();
        }
        path_candidates(raw, &scope.module)
            .into_iter()
            .filter_map(|path| {
                let (name, module) = path.split_last()?;
                self.module_declaration_sites
                    .get(&(module.to_vec(), name.clone()))
            })
            .flatten()
            .filter(|(boundary, _)| scope.module.starts_with(boundary))
            .map(|(_, index)| *index)
            .collect()
    }

    fn ranged_shadow(&self, module: &[String], name: &str, position: usize) -> bool {
        self.ranged_shadows
            .iter()
            .any(|(bound_module, bound, start, end)| {
                bound_module == module && bound == name && (*start..*end).contains(&position)
            })
    }
}

fn rooted_target_present(raw: &[String]) -> bool {
    match raw.first().map(String::as_str) {
        Some(ROOT_COMPONENT | SELF_COMPONENT) => raw.get(FIRST_COMPONENT_OFFSET).is_some(),
        Some(PARENT_COMPONENT) => raw.iter().any(|part| part.as_str() != PARENT_COMPONENT),
        Some(_) | None => false,
    }
}
