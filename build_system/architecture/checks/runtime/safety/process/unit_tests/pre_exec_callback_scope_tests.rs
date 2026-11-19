use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

#[test]
fn callback_binding_requires_a_named_assignment_in_the_current_statement() {
    for (text, expected) in [
        ("let callback = ||", Some("callback")),
        ("let previous = other; ||", None),
        ("= ||", None),
        ("||", None),
        ("previous(); { let callback = move ||", Some("callback")),
        ("{ previous(); } let callback = async ||", Some("callback")),
    ] {
        let tokens = tokenize(text);
        let closure = tokens.iter().position(|token| token.symbol("|")).unwrap();
        assert_eq!(
            binding_name(&tokens, 0, closure).map(|(name, _)| name),
            expected,
            "{text}"
        );
    }
    let tokens = tokenize("let callback = |unclosed");
    assert!(closure_bindings(&tokens, 0..tokens.len()).is_empty());
}

#[test]
fn latest_preceding_closure_wins_without_adopting_later_or_unrelated_bindings() {
    for (text, expected) in [
        ("owner.pre_exec(callback);", None),
        (
            "fn launch() { owner.pre_exec(callback); let callback = || late(); }",
            None,
        ),
        (
            "fn launch() { let unrelated = || wrong(); owner.pre_exec(callback); }",
            None,
        ),
        (
            "fn launch() { let callback = || first(); let callback = || second(); owner.pre_exec(callback); }",
            Some("second"),
        ),
        (
            "fn callback() { named(); } fn launch() { owner.pre_exec(callback); }",
            Some("named"),
        ),
        (
            "fn callback() { named(); } fn launch() { let callback = || local(); owner.pre_exec(callback); }",
            Some("local"),
        ),
    ] {
        let source = RustSource::parse(text);
        let found = bodies(&source);
        assert_eq!(found.len(), usize::from(expected.is_some()), "{text}");
        if let Some(expected) = expected {
            assert!(
                source.tokens[found[0].clone()]
                    .iter()
                    .any(|token| token.identifier(expected)),
                "{text}"
            );
        }
    }
}
