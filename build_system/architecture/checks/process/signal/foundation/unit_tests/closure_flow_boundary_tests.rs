use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

const ORIGIN: &str = "origin";

#[test]
fn closure_invocations_map_mismatched_arity_conservatively_without_inventing_empty_arguments() {
    for (text, expected) in [
        (
            "let callback = |first, second| {}; callback(origin);",
            vec!["first", "second"],
        ),
        ("let callback = |first| {}; callback();", vec![]),
        ("let callback = |first| {}; callback;", vec![]),
        ("let callback = |first| {}; callback(", vec![]),
        (
            "let callback = |first, second| {}; callback(other, origin);",
            vec!["second"],
        ),
    ] {
        let tokens = tokenize(text);
        let found = closures::flows(&tokens, &(0..tokens.len()));
        let mut names = found
            .iter()
            .filter(|flow| {
                tokens[flow.source.clone()]
                    .iter()
                    .any(|token| token.identifier(ORIGIN))
            })
            .flat_map(|flow| tokens[flow.pattern.clone()].iter())
            .filter_map(yolo_architecture_syntax::rust_source::identifier)
            .collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names, expected, "{text}");
    }
}

#[test]
fn closure_opening_contexts_and_incomplete_bodies_are_bounded() {
    for prefix in ["(", "[", "{", ",", "=", ";", "move", "async", "return"] {
        let tokens = tokenize(&format!("{prefix} |target| value"));
        assert!(
            !closures::flows(&tokens, &(0..tokens.len())).is_empty(),
            "{prefix}"
        );
    }
    for text in ["|target| value", "value | target", "return |unclosed"] {
        let tokens = tokenize(text);
        assert!(
            closures::flows(&tokens, &(0..tokens.len())).is_empty(),
            "{text}"
        );
    }
    for text in [
        "(|target| value)",
        "[|target| value]",
        "{|target| value}",
        "return |target| { incomplete",
    ] {
        let tokens = tokenize(text);
        for flow in closures::flows(&tokens, &(0..tokens.len())) {
            assert!(flow.pattern.start <= flow.pattern.end && flow.pattern.end <= tokens.len());
            assert!(flow.source.start <= flow.source.end && flow.source.end <= tokens.len());
        }
    }
}
