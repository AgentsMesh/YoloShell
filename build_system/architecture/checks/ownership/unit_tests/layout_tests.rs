use super::support::{inspect, messages, source};
use crate::constants::PRODUCTION_SOURCE_LAYOUT_MESSAGE;

const PACKAGE: &str = "build_system/model";

fn inspect_path(relative: &str) -> Vec<yolo_architecture_foundation::model::Violation> {
    inspect(vec![
        source(
            &format!("{PACKAGE}/BUILD.bazel"),
            format!("rust_library(name = \"library\", srcs = [\"{relative}\"])"),
        ),
        source(&format!("{PACKAGE}/{relative}"), "pub struct Model;"),
    ])
}

#[test]
fn package_source_directory_allows_nested_modules() {
    for relative in ["src/lib.rs", "src/domain/value.rs"] {
        let violations = inspect_path(relative);
        assert!(violations.is_empty(), "{}", messages(&violations));
    }
}

#[test]
fn package_roots_and_nested_source_lookalikes_are_rejected() {
    for relative in ["lib.rs", "implementation/src/lib.rs", "src_extra/lib.rs"] {
        let violations = inspect_path(relative);
        assert_eq!(violations.len(), 1, "{}", messages(&violations));
        assert!(messages(&violations).contains(PRODUCTION_SOURCE_LAYOUT_MESSAGE));
    }
}

#[test]
fn source_root_is_relative_to_nearest_package_not_any_ancestor() {
    let violations = inspect(vec![
        source(
            "build_system/model/BUILD.bazel",
            "filegroup(name = \"parent\")",
        ),
        source(
            "build_system/model/src/component/BUILD.bazel",
            "rust_library(name = \"library\", srcs = [\"lib.rs\"])",
        ),
        source(
            "build_system/model/src/component/lib.rs",
            "pub struct Model;",
        ),
    ]);
    assert_eq!(violations.len(), 1, "{}", messages(&violations));
    assert!(messages(&violations).contains(PRODUCTION_SOURCE_LAYOUT_MESSAGE));
}

#[test]
fn missing_package_is_reported_by_the_ownership_rule() {
    let violations = inspect(vec![source("build_system/orphan.rs", "pub struct Orphan;")]);
    assert_eq!(violations.len(), 1, "{}", messages(&violations));
    assert!(messages(&violations).contains(crate::constants::MISSING_OWNER_MESSAGE));
}

#[test]
fn layout_rule_does_not_reclassify_test_owned_sources_or_core() {
    let violations = inspect(vec![
        source(
            "build_system/model/BUILD.bazel",
            "rust_test(name = \"test\", srcs = [\"unit_tests/case.rs\"])",
        ),
        source(
            "build_system/model/unit_tests/case.rs",
            "#[test] fn case() {}",
        ),
        source(
            "core/crates/model/BUILD.bazel",
            "rust_library(name = \"library\", srcs = [\"lib.rs\"])",
        ),
        source("core/crates/model/lib.rs", "pub struct Model;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}
