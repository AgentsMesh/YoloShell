use super::*;

const FIRST: &str = "FIRST";
const SECOND: &str = "SECOND";

#[test]
fn command_attribute_references_without_assignments_do_not_execute() {
    for attribute in STARLARK_COMMAND_ATTRIBUTES {
        assert!(execution_lines(attribute).is_empty());
        assert!(execution_lines(&format!("observe({attribute})")).is_empty());
    }
}

#[test]
fn expression_ends_at_unmatched_delimiters_and_uncontinued_newlines() {
    for text in [")", "]", "}", ","] {
        let tokens = tokenize(text);
        assert_eq!(expression_end(&tokens, 0), 0, "{text}");
    }
    assert_eq!(expression_end(&[], 0), 0);
    let continued = tokenize("FIRST +\nSECOND");
    assert_eq!(expression_end(&continued, 0), continued.len());
    let separate = tokenize("FIRST\nSECOND");
    assert_eq!(expression_end(&separate, 0), 1);
}

#[test]
fn only_parenthesized_plus_joins_values_into_a_single_shell_word() {
    let values = BTreeMap::from([(FIRST.into(), "car".into()), (SECOND.into(), "go".into())]);
    assert_eq!(
        expression_value(&tokenize("FIRST + (SECOND)"), &values),
        crate::policy_constants::CARGO_EXECUTABLE
    );
    assert_eq!(
        expression_value(&tokenize("FIRST + unknown(SECOND)"), &values),
        "car;go"
    );
    assert_eq!(
        expression_value(&tokenize("[FIRST, SECOND]"), &values),
        "car;go"
    );
}
