use std::collections::BTreeSet;
use yolo_architecture_process_signal_foundation::process_signal_retained_bindings::Context;
use yolo_architecture_syntax::rust_source::RustSource;

const METHOD: &str = "probe";

#[test]
fn retained_receivers_require_binding_type_or_exact_self_field_ownership() {
    for (text, bound, expected) in [
        (
            "fn operation() { retained.probe(); }",
            Some("retained"),
            true,
        ),
        (
            "fn operation() { unrelated.probe(); }",
            Some("retained"),
            false,
        ),
        (
            "fn operation() { RetainedExactProcess.probe(); }",
            None,
            true,
        ),
        (
            "impl RetainedExactProcess { fn operation(&self) { self.probe(); } }",
            None,
            true,
        ),
        (
            "impl Other { fn operation(&self) { self.probe(); } }",
            None,
            false,
        ),
        ("fn operation() { self.probe(); }", None, false),
        (
            "struct Owner { retained: RetainedExactProcess } impl Owner { fn operation(&self) { self.retained.probe(); } }",
            None,
            true,
        ),
        (
            "struct Owner { retained: RetainedExactProcess } impl Owner { fn operation(&self) { other.retained.probe(); } }",
            None,
            false,
        ),
        (
            "struct Owner { retained: RetainedExactProcess } impl Owner { fn operation(&self) { retained.probe(); } }",
            None,
            false,
        ),
        (
            "struct Owner { retained: RetainedExactProcess } fn operation() { self.retained.probe(); }",
            None,
            false,
        ),
        ("fn operation() { (5).probe(); }", None, false),
        (
            "fn operation() { retained::probe(); }",
            Some("retained"),
            false,
        ),
    ] {
        let source = RustSource::parse(text);
        let context = Context::new(&source);
        let call = source
            .tokens
            .iter()
            .position(|token| token.identifier(METHOD))
            .unwrap();
        let function = source.function_at(call).unwrap();
        let bindings = bound
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            context.receiver_is_retained(&source.tokens, call, function, &bindings),
            expected,
            "{text}"
        );
        assert!(!context.receiver_is_retained(&source.tokens, 0, function, &bindings));
    }
}

#[test]
fn associated_authority_uses_exact_aliases_not_same_named_methods() {
    for (text, expected) in [
        ("RetainedExactProcess::probe()", true),
        (
            "use process::RetainedExactProcess as Alias; Alias::probe()",
            true,
        ),
        ("Other::probe()", false),
        ("RetainedExactProcess.probe()", false),
        ("probe()", false),
    ] {
        let source = RustSource::parse(text);
        let context = Context::new(&source);
        let call = source
            .tokens
            .iter()
            .position(|token| token.identifier(METHOD))
            .unwrap();
        assert_eq!(
            context.is_associated(&source.tokens, call),
            expected,
            "{text}"
        );
        assert_eq!(
            context.canonical_name("RetainedExactProcess"),
            Some("RetainedExactProcess")
        );
        assert_eq!(context.canonical_name("Other"), None);
    }
}
