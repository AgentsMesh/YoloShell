use super::*;
use yolo_architecture_process_signal_path_analysis::types::type_names;
use yolo_architecture_syntax::targeted_tokens::tokenize;

const SOURCE: &str = "core/src/macro_fixture.rs";
const FIRST_TOKEN: usize = 0;

#[test]
fn incomplete_macro_definitions_fail_closed_without_classifying_unrelated_invocations() {
    for (text, expected) in [
        ("macro_rules! name", true),
        ("macro_rules! name {", true),
        ("invoke! name", false),
        ("invoke! (Other)", false),
        ("invoke! { RetainedExactProcess }", true),
        ("invoke! [RetainedExactProcess]", true),
    ] {
        let source = RustSource::parse(text);
        let retained = type_names(&source.tokens);
        let file = SourceFile {
            absolute: SOURCE.into(),
            relative: SOURCE.into(),
            text: text.into(),
        };
        let mut violations = Vec::new();
        inspect(&file, &source, &retained, &mut violations);
        assert_eq!(!violations.is_empty(), expected, "{text}");
    }
}

#[test]
fn dynamic_type_exports_require_visibility_and_an_exported_type_reference() {
    for (text, expected) in [
        ("($item:ty) => { pub type Exposed = $item; }", true),
        ("($item:path) => { pub type Exposed = $item; }", true),
        (
            "($item:tt, $access:vis) => { $access type Exposed = $item; }",
            true,
        ),
        ("($item:ty) => { type Hidden = $item; }", false),
        (
            "($item:ty) => { impl $item { pub fn ordinary() {} } }",
            false,
        ),
        ("($item:ident) => { pub type Exposed = $item; }", false),
    ] {
        let tokens = tokenize(text);
        assert_eq!(
            unprovable_export(&tokens, FIRST_TOKEN, tokens.len()).is_some(),
            expected,
            "{text}"
        );
    }
    for text in ["", "value", "(incomplete", "[incomplete", "{incomplete"] {
        assert!(delimited(&tokenize(text), FIRST_TOKEN).is_none());
    }
}
