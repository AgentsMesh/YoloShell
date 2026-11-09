use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Match {
    No,
    Aggregate,
    Canonical,
    Uncertain,
}

impl Match {
    pub fn present(self) -> bool {
        matches!(self, Self::Aggregate | Self::Canonical)
    }

    pub(super) fn retained(canonical: bool) -> Self {
        if canonical {
            Self::Canonical
        } else {
            Self::Aggregate
        }
    }
}

pub(super) fn matched_kind(kinds: impl Iterator<Item = bool>) -> Match {
    let values = kinds.collect::<BTreeSet<_>>();
    let mut values = values.into_iter();
    match (values.next(), values.next()) {
        (Some(canonical), None) => Match::retained(canonical),
        (None, _) => Match::No,
        _ => Match::Uncertain,
    }
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct ModuleAlias {
    pub destination: Vec<String>,
    pub target: Vec<String>,
    pub visible_from: Vec<String>,
    pub module: bool,
    pub canonical: bool,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct RetainedEntry {
    pub path: Vec<String>,
    pub visible_from: Vec<String>,
    pub canonical: bool,
    pub declaration: bool,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct ScopedType {
    pub visible_from: Vec<String>,
    pub canonical: bool,
    pub declaration: bool,
}

type ScopedMatches = HashMap<Vec<String>, HashMap<Vec<String>, Match>>;

#[derive(Default)]
pub(super) struct ResolutionCache {
    modules: ScopedMatches,
    types: ScopedMatches,
}

impl ResolutionCache {
    pub fn get(&self, path: &[String], scope: &[String], module: bool) -> Option<Match> {
        self.matches(module).get(path)?.get(scope).copied()
    }

    pub fn insert(&mut self, path: &[String], scope: &[String], module: bool, matched: Match) {
        self.matches_mut(module)
            .entry(path.to_vec())
            .or_default()
            .insert(scope.to_vec(), matched);
    }

    pub fn clear(&mut self) {
        self.modules.clear();
        self.types.clear();
    }

    fn matches(&self, module: bool) -> &ScopedMatches {
        if module { &self.modules } else { &self.types }
    }

    fn matches_mut(&mut self, module: bool) -> &mut ScopedMatches {
        if module {
            &mut self.modules
        } else {
            &mut self.types
        }
    }
}

#[derive(Default)]
pub struct Catalog {
    pub(super) entries: BTreeSet<RetainedEntry>,
    pub(super) aliases: BTreeSet<ModuleAlias>,
    pub(super) direct_types: HashMap<String, BTreeSet<RetainedEntry>>,
    pub(super) exact_scopes: HashMap<Vec<String>, BTreeSet<ScopedType>>,
    pub(super) module_scopes: HashMap<Vec<String>, BTreeSet<Vec<String>>>,
    pub(super) aliases_by_destination: HashMap<Vec<String>, BTreeSet<ModuleAlias>>,
    pub(super) module_aliases_by_prefix: HashMap<Vec<String>, BTreeSet<ModuleAlias>>,
    pub(super) module_child_names: HashMap<Vec<String>, BTreeSet<String>>,
    pub(super) type_child_names: HashMap<Vec<String>, BTreeSet<String>>,
    pub(super) resolution_cache: RefCell<ResolutionCache>,
}

impl Clone for Catalog {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            aliases: self.aliases.clone(),
            direct_types: self.direct_types.clone(),
            exact_scopes: self.exact_scopes.clone(),
            module_scopes: self.module_scopes.clone(),
            aliases_by_destination: self.aliases_by_destination.clone(),
            module_aliases_by_prefix: self.module_aliases_by_prefix.clone(),
            module_child_names: self.module_child_names.clone(),
            type_child_names: self.type_child_names.clone(),
            resolution_cache: RefCell::default(),
        }
    }
}

impl PartialEq for Catalog {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries && self.aliases == other.aliases
    }
}

impl Eq for Catalog {}
