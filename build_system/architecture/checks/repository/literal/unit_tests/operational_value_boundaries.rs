use super::*;

#[test]
fn path_contracts_exclude_root_dynamic_paths_labels_and_whitespace() {
    for value in [
        "",
        "/",
        "one two.json",
        "${OUTPUT}/Yolo.app",
        "$(OUTPUT)/Yolo.app",
        "//app:target",
        "@repo//:target",
        ":target",
        "plain",
    ] {
        assert!(!fixed_path(value), "{value}");
    }
    for value in [
        "/app",
        "./app",
        "../app",
        "nested/Yolo.APP",
        "nested/config.JSON",
    ] {
        assert!(fixed_path(value), "{value}");
    }
}

#[test]
fn environment_contracts_require_both_prefix_and_identifier_shape() {
    for value in [
        "BUILD_OUTPUT",
        "RUNFILES_DIR",
        "TEST_TMPDIR",
        "TMPDIR",
        "YOLO_2",
    ] {
        assert!(environment_name(value), "{value}");
    }
    for value in ["BUILD_output", "YOLO_INVALID-VALUE", "OTHER_NAME", "YOLO_"] {
        let expected = value == "YOLO_";
        assert_eq!(environment_name(value), expected, "{value}");
    }
}

#[test]
fn declaration_attribute_exemptions_respect_source_language() {
    let mut file = SourceFile { absolute: "source.rs".into(), relative: "build_system/release/source.rs".into(), text: "#[cfg_attr(test, path = \"fixture.rs\")]\n#[cfg(feature = \"test-mode\")]\n#[cfg(unix)]".into() };
    assert!(rust_path_attribute(&file, 1));
    assert!(rust_cfg_feature(&file, 2));
    assert!(!rust_cfg_feature(&file, 3));
    file.relative = "plugins/macos/Tests/E2E/source.swift".into();
    assert!(!rust_path_attribute(&file, 1));
    assert!(!rust_cfg_feature(&file, 2));
    assert_eq!(string_literals(&file).len(), 2);
}

#[test]
fn test_paths_are_language_aware_and_suffixed_test_files_remain_exempt() {
    for (path, expected) in [
        ("build_system/release/unit_tests/check.rs", true),
        ("build_system/release/check_tests.rs", true),
        ("plugins/macos/Tests/E2E/Check.swift", true),
        ("plugins/macos/Sources/YoloWorkspace/Check.swift", false),
        ("plugins/macos/Tests/E2E/check.sh", false),
    ] {
        let file = SourceFile {
            absolute: path.into(),
            relative: path.into(),
            text: String::new(),
        };
        assert_eq!(rust_swift_test(&file), expected, "{path}");
    }
}

#[test]
fn load_exemptions_end_at_the_matching_statement_boundary() {
    let file = SourceFile {
        absolute: "BUILD.bazel".into(),
        relative: "build_system/release/BUILD.bazel".into(),
        text: "load(\n    \"//build:defs.bzl\",\n    \"rule\",\n)\nrule(name = \"build\")".into(),
    };
    assert!(!starlark_load(&file, 0));
    for line in [1, 2, 3, 4] {
        assert!(starlark_load(&file, line));
    }
    assert!(!starlark_load(&file, 5));
}

#[test]
fn schema_and_protocol_names_do_not_require_an_adjacent_marker() {
    let file = SourceFile {
        absolute: "source.rs".into(),
        relative: "build_system/release/source.rs".into(),
        text: "consume(\"schema-v2\")".into(),
    };
    assert!(high_risk_string(&file, 1, "schema-v2"));
    assert!(contract_named_value("Protocol-V2"));
    assert!(!contract_named_value("release-v2"));
    assert!(!contract_value("schema version 2"));
    assert!(!contract_value(""));
}
