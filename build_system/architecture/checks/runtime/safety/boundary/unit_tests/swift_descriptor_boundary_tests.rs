use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

#[test]
fn pipe_capacity_detection_requires_the_same_descriptor_declaration() {
    for (text, expected) in [
        ("let descriptors = [Int32](repeating: -1, count: 2)", true),
        (
            "let descriptors = [Int32](repeating: -1, count: 1 + 1,)",
            true,
        ),
        ("let unrelated = [Int32](repeating: -1, count: 2)", false),
        ("let descriptors = [Other](repeating: -1, count: 2)", false),
        (
            "let descriptors = [Int32]; let unrelated = Other(count: 2)",
            false,
        ),
        ("let descriptors = [Int32]; { Other(count: 2) }", false),
        ("let descriptors = [Int32](count = 2)", false),
        ("let descriptors = [Int32](count: 3)", false),
        ("let descriptors = [Int32](count:", false),
        ("let descriptors = [Int32](count:)", false),
    ] {
        let tokens = tokenize(text);
        let index = tokens
            .iter()
            .position(|token| token.identifier("count"))
            .unwrap();
        assert_eq!(
            raw_descriptor_count(&tokens, index).is_some(),
            expected,
            "{text}"
        );
    }
}

#[test]
fn raw_descriptor_index_detection_checks_delimiters_casts_and_complete_expressions() {
    for (text, expected) in [
        ("descriptors[0]", true),
        ("descriptors[1 as Int]", true),
        ("descriptors[1 - 1]", true),
        ("descriptors[2]", false),
        ("other[0]", false),
        ("descriptors(0)", false),
        ("descriptors[0", false),
        ("descriptors[]", false),
        ("descriptors[unknown]", false),
        ("descriptors", false),
    ] {
        let tokens = tokenize(text);
        assert_eq!(raw_index(&tokens, 0).is_some(), expected, "{text}");
    }
    let tokens = tokenize("count");
    assert!(raw_descriptor_count(&tokens, 0).is_none());
}
