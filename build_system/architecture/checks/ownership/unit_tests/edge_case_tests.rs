use super::support::{inspect, messages, source};

#[test]
fn build_shorthand_and_local_file_labels_are_owned() {
    let build = r#"
rust_binary(name = "tool", srcs = [":src/main.rs", ":src/main.rs"])
"#;
    let violations = inspect(vec![
        source("build_system/tool/BUILD", build),
        source("build_system/tool/src/main.rs", "fn main() {}"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn glob_exclusions_leave_a_production_source_unowned() {
    let build = r#"
rust_library(
    name = "library",
    srcs = glob(["src/*.rs"], exclude = ["src/orphan.rs"]),
)
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub mod orphan;"),
        source("build_system/model/src/orphan.rs", "pub struct Orphan;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("src/orphan.rs"), "{report}");
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn cyclic_filegroups_fail_closed_without_recursion() {
    let build = r#"
filegroup(name = "first", srcs = [":second"])
filegroup(name = "second", srcs = [":first"])
rust_library(name = "library", srcs = [":first"])
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn incomplete_rules_cannot_claim_sources() {
    let build = "rust_library(name = \"library\")";
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn path_attributes_cannot_escape_the_repository() {
    let build = r#"rust_library(name = "library", srcs = ["src/lib.rs"])"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source(
            "build_system/model/src/lib.rs",
            "#[path = \"../../../../outside.rs\"] mod outside;",
        ),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn reviewed_schema_macro_owns_only_its_declared_handwritten_root() {
    use super::constants::{CAPNP_DEFINITIONS, CAPNP_GENERATION};
    let inventory = || {
        vec![
            source(
                crate::OWNERSHIP_POLICY_BUILD_FILE,
                "",
            ),
            source(
                "core/crates/other/BUILD.bazel",
                "load(\"//build_system/bazel/capnp:defs.bzl\", schema = \"capnp_rust_library\")\nschema(name=\"schema\",srcs=[\"model.capnp\"],crate_root=\"src/lib.rs\")",
            ),
            source("build_system/bazel/capnp/defs.bzl", CAPNP_DEFINITIONS),
            source("build_system/bazel/capnp/generation.bzl", CAPNP_GENERATION),
            source("core/crates/other/src/lib.rs", "pub struct Authored;"),
        ]
    };
    let admitted = inspect(inventory());
    assert!(admitted.is_empty(), "{}", messages(&admitted));
    let mut files = inventory();
    files.push(source(
        "core/crates/other/src/orphan.rs",
        "pub struct Orphan;",
    ));
    assert!(
        messages(&inspect(files)).contains(crate::constants::MISSING_OWNER_MESSAGE)
    );
}
