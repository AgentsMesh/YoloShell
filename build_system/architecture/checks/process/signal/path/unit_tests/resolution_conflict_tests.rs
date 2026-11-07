use super::*;

const INDEX: usize = 3;
const NAME: &str = "Alias";

#[test]
fn canonical_aggregate_and_clean_evidence_conflicts_always_preserve_uncertainty() {
    let boundaries = VisibilityBoundaries::from([vec!["root".into()]]);
    for first in ["canonical", "aggregate", "clean"] {
        for second in ["canonical", "aggregate", "clean"] {
            let mut result = Resolution::default();
            for kind in [first, second] {
                match kind {
                    "canonical" => {
                        result.bind_canonical(INDEX);
                    }
                    "aggregate" => {
                        result.bind_aggregate(INDEX, &boundaries);
                    }
                    "clean" => {
                        result.bind_clean(INDEX);
                    }
                    _ => unreachable!(),
                }
            }
            assert_eq!(result.uncertain, first != second, "{first}, {second}");
            assert_eq!(
                result.indices.contains(&INDEX),
                first != "clean" || second != "clean"
            );
        }
    }
}

#[test]
fn type_registration_rejects_missing_aggregate_boundaries_and_identity_collisions() {
    let boundaries = VisibilityBoundaries::from([vec!["root".into()]]);
    for canonical_first in [false, true] {
        let mut result = Resolution::default();
        assert!(result.bind_type(INDEX, NAME.into(), canonical_first, Some(&boundaries)));
        assert!(!result.uncertain);
        assert!(!result.bind_type(INDEX, NAME.into(), canonical_first, Some(&boundaries)));
        assert!(result.bind_type(INDEX, NAME.into(), !canonical_first, Some(&boundaries)));
        assert!(result.uncertain);
    }
    let mut result = Resolution::default();
    assert!(result.bind_type(INDEX, NAME.into(), false, None));
    assert!(result.uncertain);
    assert!(result.names.contains(NAME));
    assert!(result.canonical_names.is_empty());
}

#[test]
fn independent_declaration_and_visibility_bindings_are_idempotent() {
    let mut result = Resolution::default();
    for expected in [true, false] {
        assert_eq!(result.bind_declaration(INDEX, INDEX), expected);
        assert_eq!(
            result.bind_module(INDEX, NAME.into(), vec![NAME.into()], Vec::new()),
            expected
        );
        assert_eq!(
            result.bind_type_alias(INDEX, NAME.into(), vec![NAME.into()], Vec::new(), true),
            expected
        );
    }
    assert_eq!(result.module_bindings.len(), 1);
    assert_eq!(result.type_bindings.len(), 1);
    assert!(!result.uncertain);
}
