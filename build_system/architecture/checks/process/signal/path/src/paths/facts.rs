use super::aliases::{Aliases, Scope};
use super::{Resolution, import_ranges, scopes};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use yolo_architecture_build_graph::modules::ModuleIndex;
use yolo_architecture_syntax::targeted_tokens::Token;

pub struct ResolutionFacts {
    pub(super) imports: Vec<Range<usize>>,
    pub(super) aliases: Aliases,
    pub(super) base: Resolution,
    scope_cache: RefCell<HashMap<usize, Rc<Vec<Scope>>>>,
}

impl Clone for ResolutionFacts {
    fn clone(&self) -> Self {
        Self {
            imports: self.imports.clone(),
            aliases: self.aliases.clone(),
            base: self.base.clone(),
            scope_cache: RefCell::default(),
        }
    }
}

pub fn prepare(relative: &str, tokens: &[Token], modules: &ModuleIndex) -> ResolutionFacts {
    let mut aliases = Aliases::default();
    let mut base = Resolution::default();
    scopes::install_shadows(relative, tokens, modules, &mut aliases);
    super::externs::scan(relative, tokens, modules, &mut aliases, &mut base);
    ResolutionFacts {
        imports: import_ranges(tokens),
        aliases,
        base,
        scope_cache: RefCell::default(),
    }
}

impl ResolutionFacts {
    pub(super) fn scopes_at(
        &self,
        relative: &str,
        index: usize,
        modules: &ModuleIndex,
    ) -> Rc<Vec<Scope>> {
        if let Some(scopes) = self.scope_cache.borrow().get(&index) {
            return Rc::clone(scopes);
        }
        let scopes = Rc::new(scopes::at(relative, index, modules));
        self.scope_cache
            .borrow_mut()
            .insert(index, Rc::clone(&scopes));
        scopes
    }
}
