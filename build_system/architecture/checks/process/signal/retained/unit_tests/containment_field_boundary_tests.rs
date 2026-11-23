use super::*;
use std::collections::BTreeSet;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_build_graph::{
    crates::{Catalog, CrateRoot},
    modules::{self, ModuleIndex},
};
use yolo_architecture_process_signal_path_analysis::types::type_names;
use yolo_architecture_syntax::{rust_source::RustSource, targeted_tokens::Token};

const ROOT: &str = "core/src/lib.rs";
const CONTAINER: &str = "Container";
const FIELD: &str = "member";

#[path = "containment_context_boundaries.rs"]
mod context_boundaries;

fn with_context(text: &str, inspect: impl FnOnce(&mut Context<'_>)) {
    let file = SourceFile {
        absolute: ROOT.into(),
        relative: ROOT.into(),
        text: text.into(),
    };
    let source = RustSource::parse(text);
    let catalog = Catalog {
        roots: BTreeSet::from([CrateRoot {
            id: ROOT.into(),
            source: ROOT.into(),
            product: true,
        }]),
        unresolved: BTreeSet::new(),
    };
    let modules = modules::discover(&[(&file, RustSource::parse(text))], &catalog);
    let retained = type_names(&source.tokens);
    let mut context = Context::new(ROOT, &source.tokens, &retained, &modules);
    inspect(&mut context);
}

fn position(tokens: &[Token], name: &str) -> usize {
    tokens
        .iter()
        .position(|token| token.identifier(name))
        .unwrap()
}

#[test]
fn field_visibility_is_intersected_with_its_container_and_retained_type_audience() {
    for (container, field, expected) in [
        ("pub(super)", "pub(super)", true),
        ("", "pub(super)", true),
        ("pub", "pub", false),
        ("pub(other)", "pub(super)", false),
        ("pub(super)", "pub(other)", false),
    ] {
        let text = format!(
            "mod outer {{ mod inner {{ pub(super) struct Retained {{ value: RetainedExactProcess }} {container} struct Container {{ {field} member: Retained }} }} }}"
        );
        with_context(&text, |context| {
            let declaration = position(context.tokens, CONTAINER) - 1;
            let field = position(context.tokens, FIELD);
            let public = context.tokens[..field]
                .iter()
                .rposition(|token| token.identifier(PUBLIC_KEYWORD))
                .unwrap();
            let range = field + 2..context.tokens.len();
            assert_eq!(
                allowed(context, declaration, public, range.clone()),
                expected,
                "{text}"
            );
            assert!(!allowed(context, declaration, field, range.clone()));
            assert!(!allowed(context, declaration, usize::MAX, range.clone()));
            context.certain = false;
            assert!(!allowed(context, declaration, public, range.clone()));
            assert!(!context.allows_trait_associated(declaration, declaration, range));
        });
    }
}

#[test]
fn missing_module_ownership_and_nonpublic_surfaces_cannot_authorize_fields() {
    let text = "struct Container { pub member: RetainedExactProcess }";
    with_context(text, |context| {
        let field = position(context.tokens, FIELD);
        let public = field - 1;
        let range = field + 2..context.tokens.len();
        let empty = ModuleIndex::default();
        let detached = Context::new(ROOT, context.tokens, context.retained, &empty);
        assert!(!allowed(&detached, 0, public, range.clone()));
        assert!(!detached.allows_explicit(public, field, range.clone()));
        assert!(!context.allows_surface(0, range.clone()));
        assert!(!context.allows_range(0..0, &[]));
        assert!(!context.allows_trait_associated(0, field, range));
    });
}

#[test]
fn sibling_visibility_audiences_have_no_permitted_intersection() {
    let root = vec!["root".to_owned()];
    let left = vec!["root".to_owned(), "left".to_owned()];
    let right = vec!["root".to_owned(), "right".to_owned()];
    assert_eq!(narrowest(&left, &root), Some(left.as_slice()));
    assert_eq!(narrowest(&root, &left), Some(left.as_slice()));
    assert_eq!(narrowest(&left, &left), Some(left.as_slice()));
    assert!(narrowest(&left, &right).is_none());
}
