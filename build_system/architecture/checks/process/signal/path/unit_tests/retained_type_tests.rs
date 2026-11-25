use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

const NAME: &str = "Candidate";
const INDEX: usize = 0;
const DECLARATION: usize = 4;

#[test]
fn explicit_reference_facts_take_precedence_over_spelling_and_aggregate_names() {
    let tokens = tokenize(NAME);
    let boundaries = VisibilityBoundaries::from([vec!["root".into()]]);
    for retained in [false, true] {
        let mut types = RetainedTypes::default();
        if retained {
            types.names.insert(NAME.into());
            types.canonical_names.insert(NAME.into());
        }
        assert_eq!(types.contains(&tokens, INDEX), retained);
        assert_eq!(types.contains_canonical(&tokens, INDEX), retained);
        types.clean_qualified.insert(INDEX);
        assert!(!types.contains(&tokens, INDEX));
        assert!(!types.contains_canonical(&tokens, INDEX));
        types.qualified.insert(INDEX);
        types.canonical_qualified.insert(INDEX);
        assert!(types.contains(&tokens, INDEX));
        assert!(types.contains_canonical(&tokens, INDEX));
        types.aggregate_qualified.insert(INDEX, boundaries.clone());
        assert!(!types.contains_canonical(&tokens, INDEX));
        assert!(types.aggregate_boundaries(&tokens, INDEX).is_none());
    }
    let mut types = RetainedTypes::default();
    types.aggregate_qualified.insert(INDEX, boundaries.clone());
    assert!(!types.contains_canonical(&tokens, INDEX));
    assert_eq!(types.aggregate_boundaries(&tokens, INDEX), Some(boundaries));
}

#[test]
fn declaration_references_do_not_fall_back_to_a_same_named_retained_type() {
    let tokens = tokenize(NAME);
    for canonical in [None, Some(false), Some(true)] {
        let mut types = RetainedTypes::default();
        types.names.insert(NAME.into());
        types.canonical_names.insert(NAME.into());
        types
            .declaration_references
            .insert(INDEX, BTreeSet::from([DECLARATION]));
        if let Some(canonical) = canonical {
            types.declarations.insert(DECLARATION, canonical);
        }
        assert_eq!(types.contains(&tokens, INDEX), canonical.is_some());
        assert_eq!(
            types.contains_canonical(&tokens, INDEX),
            canonical == Some(true)
        );
        assert_eq!(types.declaration_canonical(DECLARATION), canonical);
    }
    let mut types = RetainedTypes::default();
    types.canonical_names.insert(NAME.into());
    types
        .aggregate_names
        .insert(NAME.into(), VisibilityBoundaries::new());
    assert!(!types.contains_canonical(&tokens, INDEX));
    assert!(!types.contains_canonical(&tokenize("5"), INDEX));
}

#[test]
fn malformed_struct_fields_never_derive_authority_from_neighboring_declarations() {
    for text in [
        "struct",
        "struct 5 {}",
        "struct Empty;",
        "struct Unclosed { field: Candidate",
        "struct Invalid { 5: Candidate }",
        "struct Empty {}",
    ] {
        let tokens = tokenize(text);
        assert!(
            retained_fields(&tokens, &RetainedTypes::default()).is_empty(),
            "{text}"
        );
    }
    let tokens = tokenize("struct Owner { retained: RetainedExactProcess, clean: Other }");
    let retained = type_names(&tokens);
    assert_eq!(
        retained_fields(&tokens, &retained),
        BTreeSet::from([("Owner".into(), "retained".into())])
    );
}
