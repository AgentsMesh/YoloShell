use super::*;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::{Kind, tokenize};

const REPLACEMENT: &str = "untrusted";
const MUTATION_NUMBER: u64 = 7;
const WRAPPER: &str = "wrapper";

#[test]
fn resolved_binding_requires_every_assignment_operator_and_trailer() {
    let tokens = tokenize(&format!(
        "let {} = {}.context(message)?;",
        contract::AUDIT_CALL_BINDING,
        contract::AUDIT_INPUT_BINDING
    ));
    let binding = tokens
        .iter()
        .position(|token| token.identifier(contract::AUDIT_CALL_BINDING))
        .unwrap();
    let input = tokens
        .iter()
        .position(|token| token.identifier(contract::AUDIT_INPUT_BINDING))
        .unwrap();
    assert!(resolved_binding(&tokens, binding));
    assert!(resolved_source(&tokens, input));
    for index in (0..input + 4).chain(tokens.len() - 2..tokens.len()) {
        if index == binding {
            continue;
        }
        for kind in [
            Kind::Identifier(REPLACEMENT.into()),
            Kind::Symbol(REPLACEMENT.into()),
            Kind::Number(MUTATION_NUMBER),
        ] {
            let mut changed = tokens.clone();
            changed[index].kind = kind;
            assert!(!resolved_binding(&changed, binding), "{index}");
        }
    }
    for length in binding + 1..tokens.len() {
        assert!(!resolved_binding(&tokens[..length], binding), "{length}");
    }
    assert!(!direct_call(&[], 0));
    assert!(!resolved_source(&[], 0));
}

#[test]
fn input_parameter_must_precede_the_body_and_have_the_exact_optional_type() {
    let text = format!(
        "fn {WRAPPER}({}: Option<{}>) {{ body(); }}",
        contract::AUDIT_INPUT_BINDING,
        contract::AUDIT_SIGNAL_FUNCTION
    );
    let source = RustSource::parse(&text);
    let input = source
        .tokens
        .iter()
        .position(|token| token.identifier(contract::AUDIT_INPUT_BINDING))
        .unwrap();
    let function = source
        .functions()
        .find(|function| function.name == WRAPPER)
        .unwrap();
    assert!(input_parameter(&source.tokens, function, input));
    for index in input + 1..input + 6 {
        let mut tokens = source.tokens.clone();
        tokens[index].kind = Kind::Identifier(REPLACEMENT.into());
        assert!(!input_parameter(&tokens, function, input), "{index}");
    }
    let mut tokens = source.tokens.clone();
    for token in &mut tokens {
        if token.symbol(BLOCK_OPEN) {
            token.kind = Kind::Identifier(REPLACEMENT.into());
        }
    }
    assert!(!input_parameter(&tokens, function, input));
}

#[test]
fn wrapper_binding_count_and_macro_expansion_cannot_be_bypassed() {
    let text = format!(
        "fn {WRAPPER}({}: Option<{}>) {{ let {} = {}.context(message)?; {}(identity, signal); }}",
        contract::AUDIT_INPUT_BINDING,
        contract::AUDIT_SIGNAL_FUNCTION,
        contract::AUDIT_CALL_BINDING,
        contract::AUDIT_INPUT_BINDING,
        contract::AUDIT_CALL_BINDING
    );
    for (addition, approved) in [
        (String::new(), true),
        (format!("{};", contract::AUDIT_INPUT_BINDING), false),
        (format!("{};", contract::AUDIT_CALL_BINDING), false),
        ("untrusted!();".to_owned(), false),
    ] {
        let changed = text.replacen('}', &format!("{addition}}}"), 1);
        let source = RustSource::parse(&changed);
        let function = source
            .functions()
            .find(|function| function.name == WRAPPER)
            .unwrap();
        assert_eq!(
            exact_bindings(&source.tokens, function),
            approved,
            "{addition}"
        );
    }
}
