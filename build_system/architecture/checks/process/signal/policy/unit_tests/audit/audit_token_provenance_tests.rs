use super::*;
use yolo_architecture_syntax::targeted_tokens::{Kind, tokenize};

const REPLACEMENT: &str = "untrusted";
const MUTATION_NUMBER: u64 = 7;

#[test]
fn capability_probe_requires_every_operator_and_identity() {
    let tokens = tokenize(&format!(
        "{}.get_or_init({}).is_some()",
        contract::AUDIT_STATIC_BINDING,
        contract::AUDIT_RESOLVER_FUNCTION
    ));
    assert!(exact_capability_probe(&tokens, 0));
    for index in 1..8 {
        for kind in [
            Kind::Identifier(REPLACEMENT.into()),
            Kind::Symbol(REPLACEMENT.into()),
            Kind::Number(MUTATION_NUMBER),
        ] {
            let mut changed = tokens.clone();
            changed[index].kind = kind;
            assert!(!exact_capability_probe(&changed, 0), "{index}");
        }
    }
    for length in 0..8 {
        assert!(!exact_capability_probe(&tokens[..length], 0));
    }
}

#[test]
fn launch_bound_wrapper_requires_the_exact_lazy_resolver_expression() {
    for trailing in ["", ","] {
        let tokens = tokenize(&format!(
            "{}(identity, signal, *{}.get_or_init({}){trailing})",
            contract::AUDIT_WRAPPER_FUNCTION,
            contract::AUDIT_STATIC_BINDING,
            contract::AUDIT_RESOLVER_FUNCTION
        ));
        assert!(exact_launch_bound_call(&tokens, 0));
        let dereference = tokens
            .iter()
            .position(|token| token.symbol(DEREFERENCE))
            .unwrap();
        for index in dereference..dereference + 7 {
            for kind in [
                Kind::Identifier(REPLACEMENT.into()),
                Kind::Symbol(REPLACEMENT.into()),
                Kind::Number(MUTATION_NUMBER),
            ] {
                let mut changed = tokens.clone();
                changed[index].kind = kind;
                assert!(!exact_launch_bound_call(&changed, 0), "{index}, {trailing}");
            }
        }
        for length in 1..tokens.len() {
            assert!(!exact_launch_bound_call(&tokens[..length], 0), "{length}");
        }
    }
    for text in [
        "wrapper",
        "wrapper()",
        "wrapper(value)",
        "wrapper(value, other)",
    ] {
        assert!(!exact_launch_bound_call(&tokenize(text), 0));
    }
}

#[test]
fn surrounding_statement_does_not_import_authority_from_its_neighbors() {
    for text in ["trusted; suspect;", "suspect; trusted;", "trusted; suspect"] {
        let tokens = tokenize(text);
        let index = tokens
            .iter()
            .position(|token| token.identifier("suspect"))
            .unwrap();
        assert!(!statement_contains(&tokens, index, "trusted"));
    }
    let tokens = tokenize("trusted suspect");
    assert!(statement_contains(&tokens, 1, "trusted"));
}

#[test]
fn later_wrapper_calls_cannot_authorize_an_earlier_static_reference() {
    let source = RustSource::parse(&format!(
        "fn {}() {{ {}; {}(identity, signal, value); }}",
        contract::AUDIT_ENTRY_FUNCTION,
        contract::AUDIT_STATIC_BINDING,
        contract::AUDIT_WRAPPER_FUNCTION
    ));
    let index = source
        .tokens
        .iter()
        .position(|token| token.identifier(contract::AUDIT_STATIC_BINDING))
        .unwrap();
    assert_eq!(enclosing_wrapper_call(&source, index), None);
    assert!(!definition(&tokenize(contract::AUDIT_WRAPPER_FUNCTION), 0));
}

#[test]
fn subscript_syntax_cannot_substitute_for_calling_the_lazy_audit_resolver() {
    let tokens = tokenize(&format!(
        "{}(identity, signal, *{}.get_or_init[{}])",
        contract::AUDIT_WRAPPER_FUNCTION,
        contract::AUDIT_STATIC_BINDING,
        contract::AUDIT_RESOLVER_FUNCTION
    ));
    assert!(!exact_launch_bound_call(&tokens, 0));
}
