use super::support::{inspect, messages, source};
use crate::RUNTIME_SOURCES_TARGET;

#[test]
fn unsupported_source_expressions_fail_closed() {
    let build = r#"
SELF = SELF
load(":absent.bzl", "IMPORTED")
rust_library(name = "cycle", srcs = SELF)
rust_library(name = "missing", srcs = IMPORTED)
rust_library(name = "label", srcs = [":absent"])
rust_library(name = "selected", srcs = select({"//conditions:default": ["src/lib.rs"]}))
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn invalid_assignments_rules_and_filegroups_claim_nothing() {
    let build = r#"
BROKEN =
filegroup(srcs = ["src/lib.rs"])
filegroup(name = "empty")
rust_library(name = "missing-comma" srcs = ["src/lib.rs"])
rust_library(name = "bad-list", srcs = ["src/lib.rs" "src/other.rs"])
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn trailing_assignments_and_unclosed_calls_fail_closed() {
    let violations = inspect(vec![
        source("build_system/trailing/sources.bzl", "BROKEN ="),
        source(
            "build_system/model/BUILD.bazel",
            "rust_library(name = \"library\", srcs = select({",
        ),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn glob_patterns_can_be_named_and_exact() {
    let named = r#"
PATTERNS = ["src/*.rs"]
EXCLUDED = []
rust_library(name = "named", srcs = glob(PATTERNS, exclude = EXCLUDED))
"#;
    let exact = r#"
rust_library(name = "exact", srcs = glob(["src/lib.rs"]))
"#;
    let violations = inspect(vec![
        source("build_system/named/BUILD.bazel", named),
        source("build_system/named/src/lib.rs", "pub struct Named;"),
        source("build_system/exact/BUILD.bazel", exact),
        source("build_system/exact/src/lib.rs", "pub struct Exact;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn a_root_source_group_cannot_feed_a_component_target() {
    let root = r#"
filegroup(name = "shared", srcs = ["build_system/model/src/lib.rs"])
"#;
    let component = r#"
rust_library(name = "model", srcs = ["//:shared"])
"#;
    let violations = inspect(vec![
        source("BUILD.bazel", root),
        source("build_system/model/BUILD.bazel", component),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(
        report.contains("cannot cross package boundaries"),
        "{report}"
    );
}

#[test]
fn every_non_production_test_directory_has_a_separate_owner() {
    let build = r#"
rust_library(
    name = "model",
    srcs = ["src/lib.rs"],
)
rust_test(
    name = "tests",
    srcs = [
        "integration_tests/case.rs",
        "fixtures/tool.rs",
        "feature_tests/case.rs",
    ],
)
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
        source(
            "build_system/model/integration_tests/case.rs",
            "fn case() {}",
        ),
        source("build_system/model/fixtures/tool.rs", "fn main() {}"),
        source("build_system/model/feature_tests/case.rs", "fn case() {}"),
        source("third_party/vendor/src/lib.rs", "pub struct Vendor;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn runtime_source_groups_cannot_aggregate_components() {
    let facade = format!(
        "filegroup(name = \"{RUNTIME_SOURCES_TARGET}\", \
         srcs = [\"//build_system/model:{RUNTIME_SOURCES_TARGET}\"])"
    );
    let model = format!("filegroup(name = \"{RUNTIME_SOURCES_TARGET}\", srcs = [\"src/lib.rs\"])");
    let violations = inspect(vec![
        source("build_system/facade/BUILD.bazel", facade),
        source("build_system/model/BUILD.bazel", model),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(
        report.contains("cannot cross package boundaries"),
        "{report}"
    );
}
