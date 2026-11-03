use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

#[test]
fn pre_exec_recognition_requires_a_qualified_complete_call() {
    let first_token = usize::default();
    for (text, inline, named) in [
        ("pre_exec(|| body())", 0, 0),
        ("owner.pre_exec", 0, 0),
        ("owner.pre_exec(", 0, 0),
        ("owner.pre_exec()", 0, 0),
        ("owner.pre_exec(5)", 0, 0),
        ("owner.pre_exec(|| body())", 1, 0),
        ("CommandExt::pre_exec(owner, callback)", 0, 1),
        ("owner.pre_exec(callback)", 0, 1),
        ("owner.pre_exec(|unclosed)", 0, 0),
    ] {
        let tokens = tokenize(text);
        assert_eq!(callback_bodies(&tokens).len(), inline, "{text}");
        assert_eq!(named_callbacks(&tokens).len(), named, "{text}");
    }
    for (text, expected) in [
        ("setrlimit", false),
        ("setrlimit))", false),
        ("setrlimit))()", true),
        ("other()", false),
    ] {
        assert_eq!(
            setrlimit_call(&tokenize(text), first_token),
            expected,
            "{text}"
        );
    }
}

#[test]
fn callback_argument_and_statement_boundaries_respect_nested_delimiters() {
    let tokens = tokenize("owner, nested(first, [second, third], { fourth; }), callback");
    let expected = tokens
        .iter()
        .position(|token| token.identifier("callback"))
        .unwrap();
    assert_eq!(last_argument_start(&tokens, 0..tokens.len()), expected);
    assert_eq!(callback_name(&tokens, 0..tokens.len()), Some("callback"));
    for text in ["(inner;); final", "[inner;]; final", "{ inner; }; final"] {
        let tokens = tokenize(text);
        let expected = tokens.iter().rposition(|token| token.symbol(";")).unwrap();
        assert_eq!(statement_end(&tokens, 0, tokens.len()), expected, "{text}");
    }
    let tokens = tokenize("(inner;)");
    assert_eq!(statement_end(&tokens, 0, tokens.len()), tokens.len());
    let tokens = tokenize("(first | second), [third | fourth], { fifth | sixth } |");
    assert_eq!(
        closure_close(&tokens, 0, tokens.len()),
        Some(tokens.len() - 1)
    );
    assert_eq!(closure_close(&tokens, 0, tokens.len() - 1), None);
}

#[test]
fn closure_opening_context_and_depth_counters_do_not_leak_between_groups() {
    for prefix in ["(", "[", "{", ",", "=", ";", "move", "async", "return"] {
        let tokens = tokenize(&format!("{prefix} |"));
        assert!(closure_open(&tokens, tokens.len() - 1, 0), "{prefix}");
    }
    assert!(!closure_open(&tokenize("value |"), 1, 0));
    assert!(closure_open(&tokenize("|"), 0, 0));
    assert!(!closure_open(&tokenize("value"), 0, 0));
    for text in ["()", "[]", "{}", ")", "]", "}", "value"] {
        let mut depth = Depth::default();
        for token in tokenize(text) {
            depth.observe(&token);
        }
        assert!(depth.is_top(), "{text}");
    }
    for text in ["(", "[", "{"] {
        let mut depth = Depth::default();
        depth.observe(&tokenize(text)[0]);
        assert!(!depth.is_top());
    }
}
