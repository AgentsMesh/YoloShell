use yolo_architecture_process_signal_foundation::process_signal_token_sequence as sequence;
use yolo_architecture_syntax::targeted_tokens::tokenize;

const PROBE: &str = "probe";

#[test]
fn malformed_or_unowned_macro_markers_do_not_gain_macro_argument_authority() {
    for (text, expected) in [
        ("(probe)", false),
        ("!(probe)", false),
        ("5!(probe)", false),
        ("call!(probe)", true),
        ("call!([value] probe)", true),
        ("call!(value) probe", false),
        ("call!([value) probe", true),
        ("{value} probe", false),
        ("[value] probe", false),
    ] {
        let tokens = tokenize(text);
        let index = tokens
            .iter()
            .position(|token| token.identifier(PROBE))
            .unwrap();
        assert_eq!(sequence::macro_argument(&tokens, index), expected, "{text}");
    }
}

#[test]
fn top_level_sequences_reject_nested_blocks_macros_and_partial_suffixes() {
    for (text, expected) in [
        ("{ probe . member }", true),
        ("{ { probe . member } }", false),
        ("{ call!(probe . member) }", false),
        ("{ probe . other }", false),
        ("{ probe }", false),
    ] {
        let tokens = tokenize(text);
        assert_eq!(
            sequence::contains_top_level_sequence(
                &tokens,
                0,
                tokens.len() - 1,
                &[PROBE, ".", "member"]
            ),
            expected,
            "{text}"
        );
    }
    let tokens = tokenize("{ probe }");
    assert!(!sequence::contains_top_level_sequence(
        &tokens,
        0,
        tokens.len() - 1,
        &[PROBE, ".", "member", "("]
    ));
    assert!(!sequence::preceding(&tokens, 0, &[PROBE]));
    assert!(!sequence::following(&tokens, tokens.len() - 1, &[PROBE]));
}

#[test]
fn method_and_definition_boundaries_do_not_underflow_or_accept_unclosed_calls() {
    use yolo_architecture_process_signal_foundation::process_signal_syntax as syntax;
    let bare = tokenize(PROBE);
    assert!(!syntax::definition(&bare, 0));
    assert!(!syntax::method_call(&bare, 0));
    for text in ["probe", "probe(", "probe(value"] {
        assert!(
            !syntax::call_has_argument_separator(&tokenize(text), 0),
            "{text}"
        );
    }
    for text in ["<Type as Trait>::probe", "<Type>>::probe", "Type>::probe"] {
        let tokens = tokenize(text);
        let index = tokens
            .iter()
            .position(|token| token.identifier(PROBE))
            .unwrap();
        assert_eq!(syntax::qualified_reference(&tokens, index), None, "{text}");
    }
}
