use super::*;

#[path = "crate_catalog_static_binding_tests.rs"]
mod static_binding_tests;
#[path = "crate_catalog_testonly_tests.rs"]
mod testonly_tests;

#[test]
fn resolves_main_repository_labels_from_every_build_definition() {
    let catalog = discover(&[
        source("MODULE.bazel", "module(name = \"main\")"),
        source(
            "other/BUILD.bazel",
            r#"
one(crate_root = "//core:src/attach_main.rs")
two(crate_root = "@//core:src/attach_main.rs")
three(crate_root = "@@//core:src/attach_main.rs")
package_sub(crate_root = "//core/src:attach_main.rs")
four(crate_root = "@repo//core:src/external.rs")
five(crate_root = "@@repo//core:src/external.rs")
"#,
        ),
        source(
            "outside/custom.bzl",
            "wrapped(crate_root = \"//core:src/outside_bzl.rs\")",
        ),
        source(
            "build_system/bazel/rust/custom.bzl",
            "wrapped(crate_root = \"src:relative.rs\")\nwrapped(crate_root = \":src/colon.rs\")",
        ),
    ]);
    assert_eq!(
        sources(&catalog),
        set(&[
            "core/src/attach_main.rs",
            "core/src/outside_bzl.rs",
            "build_system/bazel/rust/src/relative.rs",
            "build_system/bazel/rust/src/colon.rs",
        ]),
    );
    assert_eq!(catalog.roots.len(), 7);
    assert!(catalog.unresolved.is_empty());
    assert_eq!(
        labels::resolve("rust", "@repo//core:src/external.rs"),
        Resolution::External,
    );
}

#[test]
fn unsupported_main_repository_labels_fail_closed() {
    let catalog = one(
        "outside/BUILD",
        "rust_binary(name = \"bad\", crate_root = \"//core/src/main.rs\")",
    );
    assert!(catalog.roots.is_empty());
    assert_eq!(catalog.unresolved, set(&["outside/BUILD"]));
    assert_eq!(labels::resolve("rust", "//core:"), Resolution::Uncertain);
}

#[test]
fn incomplete_rules_without_sources_do_not_pollute_other_checks() {
    let catalog = one("tools/BUILD.bazel", "rust_test(name = \"fixture\")");
    assert!(catalog.roots.is_empty());
    assert!(catalog.unresolved.is_empty());
}

#[test]
fn infers_all_rules_rust_roots_and_preserves_test_status() {
    let catalog = one(
        "core/BUILD.bazel",
        r#"
BINARY_SRCS = [
    "src/deep/main.rs",
    "src/main.rs",
]
rust_binary(name = "binary", srcs = BINARY_SRCS)
rust_library(name = "library", srcs = ["src/deep/lib.rs", "src/lib.rs"])
rust_static_library(name = "named", srcs = ["src/named.rs", "src/other.rs"])
rust_shared_library(name = "target", crate_name = "crate", srcs = ["src/crate.rs", "src/other.rs"])
rust_proc_macro(name = "single", srcs = ["src/single.rs"])
rust_test(name = "check", srcs = ["src/check.rs"])
"#,
    );
    assert_eq!(
        sources(&catalog),
        set(&[
            "core/src/main.rs",
            "core/src/lib.rs",
            "core/src/named.rs",
            "core/src/crate.rs",
            "core/src/single.rs",
            "core/src/check.rs",
        ]),
    );
    assert!(catalog.unresolved.is_empty());
    assert!(root(&catalog, "core/src/main.rs").product);
    assert!(!root(&catalog, "core/src/check.rs").product);
}

#[test]
fn dynamic_ambiguous_and_duplicate_roots_fail_closed() {
    for (relative, definition) in [
        (
            "build_system/bazel/rust/dynamic_root.bzl",
            "rust_binary(name = \"bad\", crate_root = left + right)",
        ),
        (
            "build_system/bazel/rust/dynamic_srcs.bzl",
            "rust_binary(name = \"bad\", srcs = glob([\"src/*.rs\"]))",
        ),
        (
            "build_system/bazel/rust/combined_srcs.bzl",
            "rust_binary(name = \"bad\", srcs = [\"src/main.rs\"] + MORE)",
        ),
        (
            "build_system/bazel/rust/ambiguous_srcs.bzl",
            "rust_binary(name = \"bad\", srcs = [\"src/a/main.rs\", \"src/b/main.rs\"])",
        ),
        (
            "build_system/bazel/rust/duplicate_root.bzl",
            "rust_binary(crate_root = \"src/main.rs\", crate_root = \"src/lib.rs\")",
        ),
    ] {
        let catalog = one(relative, definition);
        assert!(catalog.roots.is_empty(), "{relative}");
        assert_eq!(catalog.unresolved, set(&[relative]));
    }
}

#[test]
fn top_level_constants_are_static_but_shadowing_is_not() {
    let catalog = one(
        "build_system/bazel/rust/constants.bzl",
        r#"
ROOT = "src/retained_main.rs"
rust_binary(name = "retained", crate_root = ROOT)
def helper():
    ROOT = "src/clean_main.rs"
"#,
    );
    assert_eq!(
        sources(&catalog),
        set(&["build_system/bazel/rust/src/retained_main.rs"])
    );
    assert!(catalog.unresolved.is_empty());
}

fn one(relative: &str, text: &str) -> Catalog {
    discover(&[source(relative, text)])
}

fn source(relative: &str, text: &str) -> SourceFile {
    SourceFile {
        absolute: std::path::PathBuf::from(relative),
        relative: relative.to_owned(),
        text: text.to_owned(),
    }
}

fn sources(catalog: &Catalog) -> BTreeSet<String> {
    catalog
        .roots
        .iter()
        .map(|root| root.source.clone())
        .collect()
}

fn root<'a>(catalog: &'a Catalog, source: &str) -> &'a CrateRoot {
    catalog
        .roots
        .iter()
        .find(|root| root.source == source)
        .expect("expected crate root")
}

fn set(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

#[path = "label_resolution_tests.rs"]
mod label_resolution_tests;
