use super::support::{inspect, messages, source};

const LIBRARY: &str = r#"
rust_library(
    name = "library",
    srcs = glob(["src/*.rs"]),
)
"#;

#[test]
fn one_real_library_owns_each_production_source() {
    let violations = inspect(vec![
        source("core/crates/model/BUILD.bazel", LIBRARY),
        source("core/crates/model/src/lib.rs", "mod value;"),
        source("core/crates/model/src/value.rs", "pub struct Value;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn static_library_is_one_explicit_production_owner_and_not_an_exemption() {
    let build = LIBRARY.replace(
        crate::constants::RUST_LIBRARY_RULE,
        crate::constants::RUST_STATIC_LIBRARY_RULE,
    );
    let files = || {
        vec![
            source("core/crates/model/BUILD.bazel", &build),
            source("core/crates/model/src/lib.rs", "pub struct Model;"),
        ]
    };
    assert!(inspect(files()).is_empty());
    let mut duplicate = files();
    duplicate.first_mut().unwrap().text.push_str(LIBRARY);
    assert!(messages(&inspect(duplicate)).contains(crate::constants::DUPLICATE_OWNER_MESSAGE));
    let missing = build.replace("src/*.rs", "missing/*.rs");
    let report = messages(&inspect(vec![
        source("core/crates/model/BUILD.bazel", missing),
        source("core/crates/model/src/lib.rs", "pub struct Model;"),
    ]));
    assert!(report.contains(crate::constants::MISSING_OWNER_MESSAGE));
}

#[test]
fn missing_and_duplicate_production_owners_are_rejected() {
    let build = r#"
rust_library(name = "first", srcs = ["src/lib.rs"])
rust_library(name = "second", srcs = ["src/lib.rs"])
"#;
    let violations = inspect(vec![
        source("core/crates/model/BUILD.bazel", build),
        source("core/crates/model/src/lib.rs", "pub mod orphan;"),
        source("core/crates/model/src/orphan.rs", "pub struct Orphan;"),
    ]);
    let report = messages(&violations);
    assert!(
        report.contains(crate::constants::DUPLICATE_OWNER_MESSAGE),
        "{report}"
    );
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn test_only_variants_do_not_duplicate_production_ownership() {
    let build = r#"
rust_library(name = "library", srcs = ["src/lib.rs"])
rust_library(name = "adversarial", testonly = True, srcs = ["src/lib.rs"])
"#;
    let violations = inspect(vec![
        source("core/crates/model/BUILD.bazel", build),
        source("core/crates/model/src/lib.rs", "pub struct Model;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn loaded_source_lists_are_resolved() {
    let build = r#"
load(":sources.bzl", "MODEL_SOURCES")
rust_library(name = "library", srcs = MODEL_SOURCES)
"#;
    let violations = inspect(vec![
        source("core/crates/model/BUILD.bazel", build),
        source(
            "core/crates/model/sources.bzl",
            "MODEL_SOURCES = [\"src/lib.rs\"]",
        ),
        source("core/crates/model/src/lib.rs", "pub struct Model;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn parent_glob_stops_at_a_component_package() {
    let parent = r#"rust_library(name = "facade", srcs = glob(["**/*.rs"]))"#;
    let component = r#"rust_library(name = "component", srcs = glob(["src/*.rs"]))"#;
    let violations = inspect(vec![
        source("build_system/release/BUILD.bazel", parent),
        source("build_system/release/src/lib.rs", "pub fn release() {}"),
        source("build_system/release/foundation/BUILD.bazel", component),
        source(
            "build_system/release/foundation/src/lib.rs",
            "pub fn digest() {}",
        ),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}
