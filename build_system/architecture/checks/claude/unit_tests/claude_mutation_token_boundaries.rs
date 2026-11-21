use super::*;

#[test]
fn mutation_borrows_require_an_actual_reference_before_the_qualified_type() {
    for (prefix, expected) in [
        ("", false),
        ("super::mutation::", false),
        ("&super::mutation::", true),
        ("owner::super::mutation::", false),
    ] {
        let text = format!("{prefix}{CLAUDE_GENERATION_MUTATION_TYPE}");
        let tokens = tokenize(&text);
        let index = tokens.len() - EXPECTED_OCCURRENCES;
        assert_eq!(
            matches_occurrence(&tokens, index, Occurrence::Borrow),
            expected,
            "{text}"
        );
    }
}

#[test]
fn parent_qualification_does_not_absorb_an_unrelated_outer_namespace() {
    let text = format!("other::super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}");
    let tokens = tokenize(&text);
    let index = tokens.len() - EXPECTED_OCCURRENCES;
    let start = qualification_start(&tokens, index).unwrap();
    assert!(tokens[start].identifier("super"));
    assert_eq!(start, 2);
}
