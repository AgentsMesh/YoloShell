use super::*;

const RETAINED: &str = "Retained";
const TRAIT: &str = "Exposes";
const UNKNOWN_TRAIT: &str = "Missing";
const IMPLEMENTATION: &str = "impl";
const OPEN: &str = "{";
const NEXT: usize = 1;
const RETAINED_DECLARATION: &str = "pub(super) struct Retained { value: RetainedExactProcess }";

#[test]
fn associated_types_require_a_resolved_trait_with_a_contained_audience() {
    for (trait_visibility, implemented, expected) in [
        ("pub(super)", TRAIT, true),
        ("pub(super)", UNKNOWN_TRAIT, false),
        ("pub", TRAIT, false),
    ] {
        let text = format!(
            "mod outer {{ mod owned {{ {RETAINED_DECLARATION} {trait_visibility} trait {TRAIT} {{ type Value; }} impl {implemented} for Owner {{ type Value = {RETAINED}; }} }} }}"
        );
        with_context(&text, |context| {
            let implementation = position(context.tokens, IMPLEMENTATION);
            let open = context.tokens[implementation..]
                .iter()
                .position(|token| token.symbol(OPEN))
                .unwrap()
                + implementation;
            let reference = context
                .tokens
                .iter()
                .rposition(|token| token.identifier(RETAINED))
                .unwrap();
            assert_eq!(
                context.allows_trait_associated(implementation, open, reference..reference + NEXT),
                expected,
                "{text}"
            );
        });
    }
}

#[test]
fn uncertain_module_identity_cannot_authorize_an_explicit_public_surface() {
    let text = format!(
        "mod outer {{ mod owned {{ {RETAINED_DECLARATION} pub(super) fn expose() -> {RETAINED} {{ loop {{}} }} }} }}"
    );
    with_context(&text, |context| {
        let public = context
            .tokens
            .iter()
            .rposition(|token| token.identifier(PUBLIC_KEYWORD))
            .unwrap();
        let reference = context
            .tokens
            .iter()
            .rposition(|token| token.identifier(RETAINED))
            .unwrap();
        let range = reference..reference + NEXT;
        assert!(context.allows_explicit(public, reference, range.clone()));
        context.certain = false;
        assert!(!context.allows_explicit(public, reference, range));
        assert!(!context.allows_declaration(position(context.tokens, RETAINED) - NEXT));
    });
}

#[test]
fn unknown_visibility_and_unresolved_local_aggregate_references_fail_closed() {
    let text = format!(
        "mod outer {{ mod owned {{ {RETAINED_DECLARATION} pub(other) fn expose() -> {RETAINED} {{ loop {{}} }} }} }}"
    );
    with_context(&text, |context| {
        let public = context
            .tokens
            .iter()
            .rposition(|token| token.identifier(PUBLIC_KEYWORD))
            .unwrap();
        let reference = context
            .tokens
            .iter()
            .rposition(|token| token.identifier(RETAINED))
            .unwrap();
        assert!(!context.allows_explicit(public, reference, reference..reference + NEXT));
        let empty = ModuleIndex::default();
        let detached = Context::new(ROOT, context.tokens, context.retained, &empty);
        assert_eq!(detached.aggregate_exposure(reference, &[]), Some(false));
    });
}

#[test]
fn aggregate_records_without_body_evidence_cannot_authorize_declarations() {
    let text =
        "mod outer { mod owned { pub(super) struct Container { field: RetainedExactProcess } } }";
    with_context(text, |context| {
        let declaration = position(context.tokens, CONTAINER) - NEXT;
        assert!(
            context
                .aggregates
                .iter()
                .any(|aggregate| aggregate.declaration == declaration)
        );
        context
            .aggregates
            .iter_mut()
            .for_each(|aggregate| aggregate.body = declaration + NEXT..declaration + NEXT);
        assert!(!context.allows_declaration(declaration));
        assert!(!context.contains_reference(declaration + NEXT..declaration + NEXT));
    });
}
