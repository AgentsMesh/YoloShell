use super::support::{inspect, messages, source};
use crate::RUNTIME_SOURCES_TARGET;

#[test]
fn source_aliases_resolve_to_the_real_owner() {
    let facade = r#"
rust_binary(name = "cli", srcs = ["//build_system/component:sources"])
"#;
    let component = format!(
        "filegroup(name = \"{RUNTIME_SOURCES_TARGET}\", srcs = [\"src/main.rs\"])\n\
         alias(name = \"sources\", actual = \":{RUNTIME_SOURCES_TARGET}\")"
    );
    let violations = inspect(vec![
        source("build_system/facade/BUILD.bazel", facade),
        source("build_system/component/BUILD.bazel", component),
        source("build_system/component/src/main.rs", "fn main() {}"),
    ]);
    let report = messages(&violations);
    assert!(!report.contains("must belong to exactly one"), "{report}");
    assert!(report.contains("root facade cannot compile"), "{report}");
}

#[test]
fn aliased_load_symbols_resolve_in_the_consuming_package() {
    let build = r#"
load(":sources.bzl", LOCAL_SOURCES = "SHARED_SOURCES")
rust_library(name = "library", srcs = LOCAL_SOURCES)
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source(
            "build_system/model/sources.bzl",
            "SHARED_SOURCES = [\"src/lib.rs\"]",
        ),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn generated_and_test_support_sources_are_not_production_owners() {
    let violations = inspect(vec![
        source(
            "build_system/model/BUILD.bazel",
            concat!(
                "rust_library(name = \"library\", srcs = [\"src/lib.rs\"])\n",
                "rust_test(name = \"tests\", srcs = [",
                "\"unit_tests/model_tests.rs\", \"test_support/support.rs\"])",
            ),
        ),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
        source("build_system/model/generated/wire.rs", "pub struct Wire;"),
        source(
            "build_system/model/unit_tests/model_tests.rs",
            "#[test] fn model() {}",
        ),
        source(
            "build_system/model/test_support/support.rs",
            "pub fn support() {}",
        ),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn recursive_source_variables_fail_closed_without_hanging() {
    let build = r#"
FIRST = SECOND
SECOND = FIRST
rust_library(name = "library", srcs = FIRST)
"#;
    let violations = inspect(vec![
        source("core/crates/model/BUILD.bazel", build),
        source("core/crates/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn absolute_file_labels_and_parenthesized_lists_are_resolved() {
    let build = r#"
SOURCES = (["//build_system/model:src/lib.rs"] + ["//build_system/model:src/value.rs"])
rust_library(name = "library", srcs = SOURCES)
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "mod value;"),
        source("build_system/model/src/value.rs", "pub struct Value;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn external_rust_labels_cannot_claim_a_local_source() {
    let build = r#"
rust_library(name = "library", srcs = ["@vendor//:lib.rs"])
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn malformed_and_external_loads_are_ignored() {
    let build = r#"
load(NAME, "UNKNOWN")
load("@vendor//:sources.bzl", "EXTERNAL")
rust_library(name = "library", srcs = ["src/lib.rs"] + UNKNOWN + EXTERNAL)
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must be statically resolvable"), "{report}");
    assert!(!report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn a_root_starlark_load_resolves_a_source_list() {
    let build = r#"
load("//:sources.bzl", "ROOT_SOURCES")
rust_library(name = "library", srcs = ROOT_SOURCES)
"#;
    let violations = inspect(vec![
        source("BUILD.bazel", build),
        source("sources.bzl", "ROOT_SOURCES = [\"lib.rs\"]"),
        source("lib.rs", "pub struct Root;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}
