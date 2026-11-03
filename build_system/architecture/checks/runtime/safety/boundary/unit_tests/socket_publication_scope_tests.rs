use super::*;

const PROBE: &str = "probe";

#[test]
fn function_context_exits_completed_bodies_but_retains_inner_blocks() {
    for (text, expected) in [
        ("probe", None),
        ("fn outer(); probe", None),
        ("fn outer() {} probe", None),
        ("fn outer() { { probe; } }", Some("outer")),
        ("fn outer() { fn inner() { probe; } }", Some("inner")),
        ("fn outer() { fn inner() {} probe; }", Some("outer")),
        ("fn 5() { probe; }", None),
    ] {
        let tokens = tokenize(text);
        let index = tokens
            .iter()
            .position(|token| token.identifier(PROBE))
            .unwrap();
        assert_eq!(enclosing_function(&tokens, index), expected, "{text}");
    }
    assert!(!is_rust("source.swift"));
}
