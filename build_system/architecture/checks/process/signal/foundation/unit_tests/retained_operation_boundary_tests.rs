use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

const SOURCE: &str = "core/src/authority_fixture.rs";
const FIRST_TOKEN: usize = 0;

#[test]
fn unscoped_and_macro_argument_authority_cannot_bypass_function_receiver_checks() {
    for (text, expected) in [
        ("RetainedExactProcess::enforce_containment;", true),
        ("target.enforce_containment();", true),
        ("fn f() { call!(enforce_containment); }", true),
        (
            "fn f(target: RetainedExactProcess) { call!(target.enforce_containment()); }",
            true,
        ),
        ("fn f() { unrelated.enforce_containment(); }", false),
        (
            "macro_rules! invoke { target.enforce_containment(); }",
            true,
        ),
        (
            "macro_rules! invoke { RetainedExactProcess::enforce_containment; }",
            true,
        ),
        ("macro_rules! invoke { Other::enforce_containment; }", false),
        ("macro_rules! invoke { unrelated(); }", false),
        (
            "fn f() { macro_rules! invoke { target.enforce_containment(); } }",
            true,
        ),
        ("invoke!(enforce_containment);", true),
    ] {
        let file = SourceFile {
            absolute: SOURCE.into(),
            relative: SOURCE.into(),
            text: text.into(),
        };
        let source = RustSource::parse(text);
        let mut violations = Vec::new();
        inspect(&file, &source, &mut violations);
        assert_eq!(!violations.is_empty(), expected, "{text}");
    }
}

#[test]
fn macro_definition_delimiters_are_bounded_and_do_not_require_braces() {
    for text in [
        "macro_rules! invoke { value }",
        "macro_rules! invoke (value)",
        "macro_rules! invoke [value]",
    ] {
        let tokens = tokenize(text);
        let body = macro_body(&tokens, FIRST_TOKEN).unwrap();
        assert_eq!(body.len(), 1);
        assert!(tokens[body.start].identifier("value"));
    }
    for text in [
        retained::MACRO_RULES,
        "macro_rules! invoke",
        "macro_rules! invoke { value",
    ] {
        assert!(macro_body(&tokenize(text), FIRST_TOKEN).is_none(), "{text}");
    }
    assert!(macro_delimiter(&tokenize("value")[FIRST_TOKEN]).is_none());
}
