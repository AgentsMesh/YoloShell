#[path = "modules/attributes.rs"]
mod attributes;
#[path = "modules/declarations.rs"]
mod declarations;
#[path = "modules/files.rs"]
mod files;
#[path = "modules/graph.rs"]
mod graph;
#[path = "modules/includes.rs"]
mod includes;
#[path = "modules/syntax.rs"]
mod syntax;

use crate::crates::Catalog as CrateCatalog;
use crate::path_syntax::module_path;
use std::collections::{BTreeMap, BTreeSet};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::Token;

const FIRST_PREFIX_LENGTH: usize = 1;

#[derive(Clone, Default, Eq, PartialEq)]
pub struct ModuleIndex {
    roots: BTreeMap<String, BTreeSet<Vec<String>>>,
    known: BTreeSet<Vec<String>>,
    uncertain: BTreeSet<String>,
    syntax: BTreeMap<String, syntax::FileIndex>,
    product: BTreeSet<String>,
    declared_crate_roots: bool,
}

pub fn discover(sources: &[(&SourceFile, RustSource)], crates: &CrateCatalog) -> ModuleIndex {
    graph::discover(sources, crates)
}

impl ModuleIndex {
    pub fn paths_at(&self, relative: &str, index: usize) -> Vec<Vec<String>> {
        let suffix = self
            .syntax
            .get(relative)
            .map(|syntax| syntax.ancestors_at(index))
            .unwrap_or_default();
        self.paths_for(relative)
            .into_iter()
            .map(|root| root.into_iter().chain(suffix.iter().cloned()).collect())
            .collect()
    }

    pub fn blocks_at(&self, relative: &str, index: usize) -> &[usize] {
        self.syntax
            .get(relative)
            .map(|syntax| syntax.blocks_at(index))
            .unwrap_or_default()
    }

    pub fn module_item(&self, relative: &str, index: usize) -> bool {
        self.syntax
            .get(relative)
            .is_some_and(|syntax| syntax.module_item(index))
    }

    pub fn contains(&self, path: &[String]) -> bool {
        self.known.contains(path)
    }

    pub fn uncertain_files(&self) -> impl Iterator<Item = &String> {
        self.uncertain.iter()
    }

    pub fn path_count(&self) -> usize {
        self.roots.values().map(BTreeSet::len).sum()
    }

    pub fn is_product(&self, relative: &str) -> bool {
        self.product.contains(relative)
    }

    pub fn uses_declared_crate_roots(&self) -> bool {
        self.declared_crate_roots
    }

    pub(super) fn inferred(relative: &str) -> Vec<String> {
        module_path(relative)
    }

    pub(super) fn paths_for(&self, relative: &str) -> Vec<Vec<String>> {
        self.roots
            .get(relative)
            .map(|paths| paths.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub(super) fn insert(&mut self, relative: &str, path: Vec<String>) -> bool {
        let mut changed = self.remember(&path);
        changed |= self
            .roots
            .entry(relative.to_owned())
            .or_default()
            .insert(path);
        changed
    }

    pub(super) fn remember(&mut self, path: &[String]) -> bool {
        (FIRST_PREFIX_LENGTH..=path.len()).fold(false, |changed, end| {
            self.known.insert(path[..end].to_vec()) || changed
        })
    }

    pub(super) fn mark_uncertain(&mut self, relative: &str) {
        self.uncertain.insert(relative.to_owned());
    }

    pub(super) fn mark_product(&mut self, relative: &str) {
        self.product.insert(relative.to_owned());
    }

    pub(super) fn index_syntax(&mut self, relative: &str, tokens: &[Token]) {
        self.syntax
            .insert(relative.to_owned(), syntax::FileIndex::build(tokens));
    }

    pub(super) fn set_declared_crate_roots(&mut self, declared: bool) {
        self.declared_crate_roots = declared;
    }
}

#[cfg(test)]
#[path = "../unit_tests/module_index_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../unit_tests/module_path_override_tests.rs"]
mod path_override_tests;
