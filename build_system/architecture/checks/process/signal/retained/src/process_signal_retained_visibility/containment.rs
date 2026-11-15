#[path = "containment/declarations.rs"]
mod declarations;
#[path = "containment/fields.rs"]
mod fields;
#[path = "containment/records.rs"]
mod records;
#[path = "containment/scope.rs"]
mod scope;

use std::collections::BTreeSet;
use std::ops::Range;
use yolo_architecture_build_graph::modules::ModuleIndex;
use yolo_architecture_process_signal_path_analysis::types::RetainedTypes;
use yolo_architecture_process_signal_path_analysis::visibility_fields as syntax;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::Token;

const BINDING_KEYWORDS: &[&str] = &[
    ALIAS_KEYWORD,
    "const",
    "enum",
    "fn",
    "static",
    "struct",
    "trait",
    "type",
    "union",
];
const ALIAS_KEYWORD: &str = "as";
const FOR_KEYWORD: &str = "for";
const NEXT_TOKEN_OFFSET: usize = 1;
const UNIQUE_OWNER_COUNT: usize = 1;

pub(super) struct Context<'a> {
    relative: &'a str,
    tokens: &'a [Token],
    retained: &'a RetainedTypes,
    modules: &'a ModuleIndex,
    aggregates: Vec<records::Aggregate>,
    local_aggregates: BTreeSet<String>,
    traits: Vec<records::Trait>,
    certain: bool,
}

impl<'a> Context<'a> {
    pub(super) fn new(
        relative: &'a str,
        tokens: &'a [Token],
        retained: &'a RetainedTypes,
        modules: &'a ModuleIndex,
    ) -> Self {
        let certain = !modules.uncertain_files().any(|file| file == relative);
        let aggregates = certain
            .then(|| records::aggregates(relative, tokens, retained, modules))
            .unwrap_or_default();
        let local_aggregates = records::local_names(tokens, retained);
        let traits = certain
            .then(|| records::traits(relative, tokens, modules))
            .unwrap_or_default();
        Self {
            relative,
            tokens,
            retained,
            modules,
            aggregates,
            local_aggregates,
            traits,
            certain,
        }
    }

    pub(super) fn allows_declaration(&self, index: usize) -> bool {
        declarations::allowed(self, index)
    }

    pub(super) fn module_item(&self, index: usize) -> bool {
        self.modules.module_item(self.relative, index)
    }

    pub(super) fn contains_reference(&self, range: Range<usize>) -> bool {
        self.references(range).next().is_some()
    }

    pub(super) fn allows_surface(&self, index: usize, range: Range<usize>) -> bool {
        let start = syntax::item_start(self.tokens, index);
        let Some(public) = syntax::visible_index(self.tokens, start, index) else {
            return false;
        };
        self.allows_explicit(public, index, range)
    }

    pub(super) fn allows_field(
        &self,
        declaration: usize,
        public: usize,
        range: Range<usize>,
    ) -> bool {
        fields::allowed(self, declaration, public, range)
    }

    pub(super) fn allows_trait_associated(
        &self,
        implementation: usize,
        open: usize,
        range: Range<usize>,
    ) -> bool {
        if !self.certain {
            return false;
        }
        let Some(trait_name) = self.trait_name(implementation, open) else {
            return false;
        };
        let Some(record) = records::resolve_trait(
            self.relative,
            self.tokens,
            trait_name,
            self.modules,
            &self.traits,
        ) else {
            return false;
        };
        self.allows_range(range, &record.visible_from)
    }

    fn allows_explicit(&self, public: usize, location: usize, range: Range<usize>) -> bool {
        if !self.certain {
            return false;
        }
        let owners = self.modules.paths_at(self.relative, location);
        if owners.len() != UNIQUE_OWNER_COUNT {
            return false;
        }
        let Some(owner) = owners.first() else {
            return false;
        };
        let Some(audience) = scope::audience(self.tokens, public, owner) else {
            return false;
        };
        self.allows_range(range, &audience)
    }

    fn allows_range(&self, range: Range<usize>, audience: &[String]) -> bool {
        let references = self.references(range).collect::<Vec<_>>();
        !references.is_empty()
            && references
                .iter()
                .all(|index| self.aggregate_exposure(*index, audience) == Some(true))
    }

    fn aggregate_exposure(&self, index: usize, audience: &[String]) -> Option<bool> {
        if let Some(aggregate) = records::resolve_aggregate(
            self.relative,
            self.tokens,
            index,
            self.modules,
            &self.aggregates,
        ) {
            return Some(audience.starts_with(&aggregate.visible_from));
        }
        if identifier(&self.tokens[index]).is_some_and(|name| self.local_aggregates.contains(name))
        {
            return Some(false);
        }
        self.retained
            .aggregate_boundaries(self.tokens, index)
            .map(|boundaries| {
                boundaries
                    .iter()
                    .all(|boundary| audience.starts_with(boundary))
            })
    }

    fn references(&self, range: Range<usize>) -> impl Iterator<Item = usize> + '_ {
        range.filter(|index| {
            self.retained.contains(self.tokens, *index) && !self.binding_name(*index)
        })
    }

    fn binding_name(&self, index: usize) -> bool {
        index
            .checked_sub(NEXT_TOKEN_OFFSET)
            .is_some_and(|previous| {
                BINDING_KEYWORDS
                    .iter()
                    .any(|keyword| self.tokens[previous].identifier(keyword))
            })
    }

    fn trait_name(&self, implementation: usize, open: usize) -> Option<usize> {
        let separator = (implementation + NEXT_TOKEN_OFFSET..open)
            .find(|index| self.tokens[*index].identifier(FOR_KEYWORD))?;
        separator
            .checked_sub(NEXT_TOKEN_OFFSET)
            .filter(|index| identifier(&self.tokens[*index]).is_some())
    }
}
