use super::{ModuleBinding, Resolution, TypeBinding, VisibilityBoundaries};

impl Resolution {
    pub(super) fn bind_type(
        &mut self,
        index: usize,
        name: String,
        canonical: bool,
        boundaries: Option<&VisibilityBoundaries>,
    ) -> bool {
        let mut changed = self.indices.insert(index) | self.names.insert(name.clone());
        if canonical {
            if self.aggregate_indices.contains_key(&index) {
                self.uncertain = true;
            }
            changed |= self.bind_canonical(index);
            changed |= self.canonical_names.insert(name);
        } else if let Some(boundaries) = boundaries {
            if self.canonical_indices.contains(&index) {
                self.uncertain = true;
            }
            changed |= self.bind_aggregate(index, boundaries);
            changed |= extend(self.aggregate_names.entry(name).or_default(), boundaries);
        } else {
            self.uncertain = true;
        }
        changed
    }

    pub(super) fn bind_aggregate(
        &mut self,
        index: usize,
        boundaries: &VisibilityBoundaries,
    ) -> bool {
        if self.clean_indices.contains(&index) {
            self.uncertain = true;
        }
        if self.canonical_indices.contains(&index) {
            self.uncertain = true;
        }
        self.indices.insert(index)
            | extend(self.aggregate_indices.entry(index).or_default(), boundaries)
    }

    pub(super) fn bind_canonical(&mut self, index: usize) -> bool {
        if self.clean_indices.contains(&index) {
            self.uncertain = true;
        }
        if self.aggregate_indices.contains_key(&index) {
            self.uncertain = true;
        }
        self.indices.insert(index) | self.canonical_indices.insert(index)
    }

    pub(super) fn bind_clean(&mut self, index: usize) -> bool {
        if self.indices.contains(&index) {
            self.uncertain = true;
        }
        self.clean_indices.insert(index)
    }

    pub(super) fn bind_declaration(&mut self, index: usize, declaration: usize) -> bool {
        self.declaration_indices
            .entry(index)
            .or_default()
            .insert(declaration)
    }

    pub(super) fn bind_module(
        &mut self,
        index: usize,
        name: String,
        target: Vec<String>,
        visible_from: Vec<String>,
    ) -> bool {
        self.module_bindings.insert(ModuleBinding {
            index,
            name,
            target,
            visible_from,
        })
    }

    pub(super) fn bind_type_alias(
        &mut self,
        index: usize,
        name: String,
        target: Vec<String>,
        visible_from: Vec<String>,
        canonical: bool,
    ) -> bool {
        self.type_bindings.insert(TypeBinding {
            index,
            name,
            target,
            visible_from,
            canonical,
        })
    }
}

fn extend(target: &mut VisibilityBoundaries, values: &VisibilityBoundaries) -> bool {
    values.iter().fold(false, |changed, value| {
        target.insert(value.clone()) || changed
    })
}

#[cfg(test)]
#[path = "../../unit_tests/resolution_conflict_tests.rs"]
mod tests;
