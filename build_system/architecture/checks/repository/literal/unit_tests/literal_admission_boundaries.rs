use yolo_architecture_foundation::SourceFile;

fn source(relative: &str, text: &str) -> SourceFile {
    SourceFile {
        absolute: relative.into(),
        relative: relative.into(),
        text: text.into(),
    }
}

#[test]
fn shell_contracts_require_named_declarations_and_unquoted_flags_are_not_exempt() {
    for (text, allowed) in [
        ("declare -r JOBS_ARGUMENT=\"--jobs=2\"", true),
        ("export RETRY_2=7", true),
        ("=\"--jobs=2\"", false),
        ("run --jobs=2", false),
    ] {
        let file = source("plugins/macos/Tests/E2E/check.sh", text);
        let mut violations = Vec::new();
        crate::inspect_file(&file, &Default::default(), &mut violations);
        assert_eq!(violations.is_empty(), allowed, "{text}: {violations:?}");
    }
}

#[test]
fn starlark_and_swift_contract_declarations_keep_language_specific_boundaries() {
    for (path, text, allowed) in [
        (
            "build_system/platform/macos/release.bzl",
            "_JOBS_2 = \"--jobs=2\"",
            true,
        ),
        ("build_system/platform/macos/release.bzl", "=\"--jobs=2\"", false),
        (
            "plugins/macos/Tests/E2E/Check.swift",
            "static let timeout = \"seconds\"",
            true,
        ),
    ] {
        let file = source(path, text);
        let mut violations = Vec::new();
        crate::inspect_file(&file, &Default::default(), &mut violations);
        assert_eq!(violations.is_empty(), allowed, "{text}: {violations:?}");
    }
}

#[test]
fn centralized_plain_words_and_unregistered_protocol_tags_respect_semantic_context() {
    let known = ["resume".to_owned()].into();
    for text in [
        "const COMMAND: &str = \"resume\";",
        "static let command = \"resume\"",
        "const TAG: &str = \"provider_session_changed\";",
        "static let tag = \"provider_session_changed\"",
    ] {
        let file = source("core/src/operation.rs", text);
        let mut violations = Vec::new();
        crate::inspect_file(&file, &known, &mut violations);
        assert_eq!(violations.len(), 1, "{text}: {violations:?}");
    }
    let file = source(
        "core/src/operation.rs",
        "println!(\"provider_session_changed\");",
    );
    let mut violations = Vec::new();
    crate::inspect_file(&file, &known, &mut violations);
    assert!(violations.is_empty());
}

#[test]
fn separated_native_test_roots_allow_only_explicit_constants_files_as_registries() {
    for (path, allowed) in [
        ("plugins/macos/unit_tests/hub/FieldsConstants.swift", true),
        ("plugins/macos/integration_tests/hub/FieldsConstants.swift", true),
        ("plugins/macos/unit_tests/hub/Fields.swift", false),
        ("plugins/macos/integration_tests/hub/Fields.swift", false),
        ("plugins/macos/integration_tests_extra/FieldsConstants.swift", false),
    ] {
        let file = source(path, "enum Fields: String { case entry = \"entry_hub\" }");
        let known = crate::registry(&[source(
            "plugins/macos/unit_tests/hub/FieldsConstants.swift",
            &file.text,
        )]);
        let mut violations = Vec::new();
        crate::inspect_file(&file, &known, &mut violations);
        assert_eq!(violations.is_empty(), allowed, "{path}: {violations:?}");
    }
}
