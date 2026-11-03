use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

const PROBE: &str = "probe";

#[test]
fn audit_unsafe_block_requires_one_direct_resolved_call() {
    let call = format!("{}(identity, signal);", contract::AUDIT_CALL_BINDING);
    for (body, valid) in [
        (format!("unsafe {{ {call} }}"), true),
        (format!("unsafe {{ {call} {call} }}"), false),
        ("unsafe { other(); }".into(), false),
        (
            format!(
                "unsafe {{ ({}) (identity, signal); }}",
                contract::AUDIT_CALL_BINDING
            ),
            false,
        ),
        (format!("unsafe {{ {call} other(); }}"), false),
        (call, false),
        ("unsafe;".into(), false),
        ("unsafe {}".into(), false),
    ] {
        let source = RustSource::parse(&format!("fn wrapper() {{ {body} }}"));
        assert_eq!(
            has_exact_unsafe_call(&source, source.functions().next().unwrap()),
            valid,
            "{body}"
        );
    }
}

#[test]
fn audit_call_shapes_distinguish_direct_qualified_and_parenthesized_references() {
    for (text, expected) in [
        ("probe()", true),
        ("module::probe()", false),
        ("owner.probe()", false),
        ("probe", false),
    ] {
        let tokens = tokenize(text);
        let index = tokens
            .iter()
            .position(|token| token.identifier(PROBE))
            .unwrap();
        assert_eq!(direct_call(&tokens, index), expected, "{text}");
    }
    for (text, expected) in [("(probe)()", true), ("(probe)", false), ("(probe);", false)] {
        let tokens = tokenize(text);
        let index = tokens
            .iter()
            .position(|token| token.symbol(CALL_CLOSE))
            .unwrap();
        assert_eq!(parenthesized_call(&tokens, index), expected, "{text}");
    }
    assert!(!direct_call(&tokenize("5()"), 0));
}

#[test]
fn wrapper_presence_and_type_cardinality_cannot_be_satisfied_by_other_owners() {
    let wrapper = format!(
        "fn {}({}: Option<{}>) {{ let {} = {}.context(message)?; unsafe {{ {}(identity, signal); }} }}",
        contract::AUDIT_WRAPPER_FUNCTION,
        contract::AUDIT_INPUT_BINDING,
        contract::AUDIT_SIGNAL_FUNCTION,
        contract::AUDIT_CALL_BINDING,
        contract::AUDIT_INPUT_BINDING,
        contract::AUDIT_CALL_BINDING
    );
    for (text, expected) in [
        (wrapper.clone(), true),
        (format!("{wrapper} {wrapper}"), false),
        (format!("impl Other {{ {wrapper} }}"), false),
        (String::new(), false),
        (
            wrapper.replace("message", contract::AUDIT_SIGNAL_FUNCTION),
            false,
        ),
    ] {
        let file = SourceFile {
            absolute: contract::MACOS_SIGNAL_PATH.into(),
            relative: contract::MACOS_SIGNAL_PATH.into(),
            text,
        };
        let mut violations = Vec::new();
        inspect_wrapper(&file, &RustSource::parse(&file.text), &mut violations);
        assert_eq!(violations.is_empty(), expected, "{}", file.text);
    }
}
