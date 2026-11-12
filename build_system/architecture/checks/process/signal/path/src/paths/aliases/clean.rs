use super::{Aliases, Scope, binding_site, bound_at_depth, explicitly_rooted};

impl Aliases {
    pub fn insert_clean(&mut self, scope: &Scope, name: String, target: Vec<String>) -> bool {
        let changed = self.insert(scope, name.clone(), target);
        self.clean_bindings
            .insert((scope.module.clone(), scope.blocks.clone(), name))
            || changed
    }

    pub fn locally_clean(&self, raw: &[String], scope: &Scope) -> bool {
        if explicitly_rooted(raw) {
            return false;
        }
        let Some((module, name, _)) = binding_site(raw, &scope.module) else {
            return false;
        };
        if module != scope.module {
            return false;
        }
        if self
            .ranged_shadows
            .iter()
            .any(|(bound_module, bound, start, end)| {
                bound_module == &module && bound == name && (*start..*end).contains(&scope.position)
            })
        {
            return true;
        }
        let Some(depth) = self.binding_depth(&module, name, &scope.blocks) else {
            return false;
        };
        bound_at_depth(&self.shadows, &module, name, &scope.blocks, depth)
            || bound_at_depth(&self.clean_bindings, &module, name, &scope.blocks, depth)
    }
}
