use crate::targeted_expression::evaluate;
use crate::targeted_tokens::tokenize;

const IDENTIFIER_EXPRESSION: &str = "FRAME_LENGTH_BYTES";
const INCOMPLETE_EXPRESSION: &str = "2 +";

#[test]
fn evaluates_supported_integer_expressions() {
    for (source, expected) in [("2 + 2", 4), ("(3 - 1) * 2", 4), ("2 << 1", 4)] {
        assert_eq!(evaluate(&tokenize(source)), Some(expected));
    }
}

#[test]
fn rejects_identifiers_and_incomplete_expressions() {
    assert_eq!(evaluate(&tokenize(IDENTIFIER_EXPRESSION)), None);
    assert_eq!(evaluate(&tokenize(INCOMPLETE_EXPRESSION)), None);
}
