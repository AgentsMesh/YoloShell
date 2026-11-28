use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

#[test]
fn include_path_requires_a_complete_macro_call_with_literal_path_evidence() {
    for (text, expected) in [
        ("include", None),
        ("include(\"tests.rs\")", None),
        ("include![\"tests.rs\"]", None),
        ("include!(\"tests.rs\"", None),
        ("include!(dynamic)", None),
        ("include!()", None),
        ("include!(\"tests.rs\")", Some("tests.rs")),
        (
            "include!(concat!(\"unit_\", \"tests.rs\"))",
            Some("unit_tests.rs"),
        ),
    ] {
        assert_eq!(
            include_path(&tokenize(text), 0).as_deref(),
            expected,
            "{text}"
        );
    }
    for (path, expected) in [
        ("tests/case.rs", true),
        ("case_tests.rs", true),
        ("case_test.rs", true),
        ("source.rs", false),
        ("/", false),
    ] {
        assert_eq!(test_path(path), expected, "{path}");
    }
}

#[test]
fn guarded_include_exemptions_do_not_cross_function_or_item_boundaries() {
    for (text, expected) in [
        (
            "#[cfg(test)] fn exercise() { include!(\"tests.rs\"); }",
            true,
        ),
        (
            "fn exercise() { #[cfg(test)] include!(\"tests.rs\"); }",
            true,
        ),
        (
            "#[cfg(test)] fn earlier() {} fn product() { include!(\"tests.rs\"); }",
            false,
        ),
        ("mod 5;", true),
        ("mod", true),
    ] {
        let file = SourceFile {
            absolute: "core/src/fixture.rs".into(),
            relative: "core/src/fixture.rs".into(),
            text: text.into(),
        };
        let mut violations = Vec::new();
        inspect(&file, &RustSource::parse(text), &mut violations);
        assert_eq!(violations.is_empty(), expected, "{text}");
    }
}
