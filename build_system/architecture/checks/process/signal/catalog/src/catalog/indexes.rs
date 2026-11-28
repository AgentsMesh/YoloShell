use super::model::ScopedType;
use super::{Catalog, ModuleAlias, RetainedEntry, alias_accessible};

impl Catalog {
    pub(super) fn index_entry(&mut self, entry: &RetainedEntry) {
        self.resolution_cache.get_mut().clear();
        self.exact_scopes
            .entry(entry.path.clone())
            .or_default()
            .insert(ScopedType {
                visible_from: entry.visible_from.clone(),
                canonical: entry.canonical,
                declaration: entry.declaration,
            });
        for end in 1..entry.path.len() {
            self.module_scopes
                .entry(entry.path[..end].to_vec())
                .or_default()
                .insert(entry.visible_from.clone());
        }
        if let Some((name, parent)) = entry.path.split_last() {
            self.type_child_names
                .entry(parent.to_vec())
                .or_default()
                .insert(name.clone());
        }
        for end in 1..entry.path.len().saturating_sub(1) {
            self.module_child_names
                .entry(entry.path[..end].to_vec())
                .or_default()
                .insert(entry.path[end].clone());
        }
    }

    pub(super) fn insert_alias(&mut self, alias: ModuleAlias) -> bool {
        let inserted = self.aliases.insert(alias.clone());
        if !inserted {
            return false;
        }
        self.resolution_cache.get_mut().clear();
        self.aliases_by_destination
            .entry(alias.destination.clone())
            .or_default()
            .insert(alias.clone());
        if alias.module {
            for end in 1..alias.destination.len() {
                self.module_aliases_by_prefix
                    .entry(alias.destination[..end].to_vec())
                    .or_default()
                    .insert(alias.clone());
                self.module_child_names
                    .entry(alias.destination[..end].to_vec())
                    .or_default()
                    .insert(alias.destination[end].clone());
            }
        } else if let Some((name, parent)) = alias.destination.split_last() {
            self.type_child_names
                .entry(parent.to_vec())
                .or_default()
                .insert(name.clone());
        }
        true
    }

    pub(super) fn aliases_for<'a>(
        &'a self,
        candidate: &[String],
        scope: &[String],
        module: bool,
    ) -> Vec<&'a ModuleAlias> {
        let mut aliases = Vec::new();
        for end in 1..=candidate.len() {
            if let Some(indexed) = self.aliases_by_destination.get(&candidate[..end]) {
                aliases.extend(
                    indexed.iter().filter(|alias| {
                        (!module || alias.module) && alias_accessible(alias, scope)
                    }),
                );
            }
        }
        if module && let Some(indexed) = self.module_aliases_by_prefix.get(candidate) {
            aliases.extend(
                indexed
                    .iter()
                    .filter(|alias| alias_accessible(alias, scope)),
            );
        }
        aliases
    }
}
