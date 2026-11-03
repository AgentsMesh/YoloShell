use yolo_architecture_policy_contracts::process_signal_retained::vocabulary::SIGNAL as METHOD;
use yolo_architecture_process_signal_foundation::process_signal_retained_receiver as receiver;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::{Kind, tokenize};

#[test]
fn receiver_boundaries_preserve_nested_groups_arrays_and_blocks() {
    for expression in [
        "holder",
        "(holder)",
        "holders[index]",
        "factory(holder)",
        "{ holder }",
    ] {
        for prefix in ["let result = ", "left + ", "left && ", "consume(left, "] {
            let source = RustSource::parse(&format!(
                "fn operation() {{ {prefix}{expression}.{METHOD}(); }}"
            ));
            let function = source.functions().next().unwrap();
            let method = source
                .tokens
                .iter()
                .position(|token| token.identifier(METHOD))
                .unwrap();
            let range = receiver::range(&source.tokens, function, method);
            let actual = source.tokens[range]
                .iter()
                .map(|token| token.kind.clone())
                .collect::<Vec<_>>();
            let expected = tokenize(expression)
                .into_iter()
                .map(|token| token.kind)
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{prefix}{expression}");
        }
    }
}

#[test]
fn every_binary_operator_separates_the_method_receiver() {
    for operator in [";", "=", ",", "&", "|", "+", "-", "*", "/", "%", "==", "!="] {
        let source = RustSource::parse(&format!(
            "fn operation() {{ previous {operator} holder.{METHOD}(); }}"
        ));
        let function = source.functions().next().unwrap();
        let method = source
            .tokens
            .iter()
            .position(|token| token.identifier(METHOD))
            .unwrap();
        let range = receiver::range(&source.tokens, function, method);
        assert_eq!(
            source.tokens[range]
                .iter()
                .map(|token| token.kind.clone())
                .collect::<Vec<_>>(),
            [Kind::Identifier("holder".to_owned())]
        );
    }
}

#[test]
fn typed_bindings_keep_nested_patterns_but_stop_at_outer_argument_boundaries() {
    for (prefix, expected) in [
        ("let binding", "binding"),
        ("arg, binding", "binding"),
        ("|binding", "binding"),
        ("(binding", "binding"),
        (";binding", "binding"),
        ("binding", "binding"),
        ("let (left, right)", "(left, right)"),
        ("arg, (left, (middle, right))", "(left, (middle, right))"),
    ] {
        let tokens = tokenize(&format!("{prefix}: Type"));
        let colon = tokens.iter().position(|token| token.symbol(":")).unwrap();
        let start = receiver::typed_binding_start(&tokens, 0, colon);
        assert_eq!(
            tokens[start..colon]
                .iter()
                .map(|token| token.kind.clone())
                .collect::<Vec<_>>(),
            tokenize(expected)
                .into_iter()
                .map(|token| token.kind)
                .collect::<Vec<_>>(),
            "{prefix}"
        );
    }
}
