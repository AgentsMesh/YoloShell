use super::support::{inspect, messages, source};

#[test]
fn root_globs_and_named_exclusions_are_resolved() {
    let build = r#"
PATTERNS = ["*.rs"]
EXCLUDED = ["orphan.rs"]
rust_library(name = "root", srcs = glob(PATTERNS, exclude = EXCLUDED) + ["orphan.rs"])
"#;
    let violations = inspect(vec![
        source("BUILD.bazel", build),
        source("lib.rs", "mod orphan;"),
        source("orphan.rs", "pub struct Orphan;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn relative_and_nested_wildcards_are_exercised() {
    let build = r#"
rust_library(name = "library", srcs = glob(["src/**/*.rs", "src/*a.rs"]))
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "mod nested;"),
        source("build_system/model/src/data.rs", "pub struct Data;"),
        source("build_system/model/src/other.rs", "pub struct Other;"),
        source(
            "build_system/model/src/nested/value.rs",
            "pub struct Value;",
        ),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn non_matching_glob_candidates_remain_explicitly_owned() {
    let build = r#"
rust_library(name = "library", srcs = glob(["src/lib.rs"]) + ["src/other.rs"])
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
        source("build_system/model/src/other.rs", "pub struct Other;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn unnamed_non_testonly_targets_still_own_their_sources() {
    let build = r#"
rust_library(testonly = False, srcs = ["src/lib.rs"])
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn unsupported_glob_patterns_cannot_hide_an_explicit_owner() {
    let build = r#"
rust_library(name = "library", srcs = glob([select({})]) + ["src/lib.rs"])
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must be statically resolvable"), "{report}");
    assert!(!report.contains("must belong to exactly one"), "{report}");
}
