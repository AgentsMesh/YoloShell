use super::support::{inspect, messages, source};

#[test]
fn cross_crate_path_attributes_are_rejected() {
    let violations = inspect(vec![
        source(
            "build_system/facade/BUILD.bazel",
            "rust_library(name = \"facade\", srcs = [\"src/lib.rs\"])",
        ),
        source(
            "build_system/facade/src/lib.rs",
            "#[path = \"../../model/src/value.rs\"] mod value;",
        ),
        source(
            "build_system/model/BUILD.bazel",
            "rust_library(name = \"model\", srcs = [\"src/value.rs\"])",
        ),
        source("build_system/model/src/value.rs", "pub struct Value;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("#[path] cannot cross"), "{report}");
}

#[test]
fn module_paths_inside_one_crate_are_admitted() {
    let violations = inspect(vec![
        source(
            "build_system/model/BUILD.bazel",
            "rust_library(name = \"model\", srcs = glob([\"src/**/*.rs\"]))",
        ),
        source(
            "build_system/model/src/lib.rs",
            "#[path = \"nested/value.rs\"] mod value;",
        ),
        source(
            "build_system/model/src/nested/value.rs",
            "pub struct Value;",
        ),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn malformed_and_non_production_path_attributes_are_ignored() {
    let violations = inspect(vec![
        source(
            "build_system/model/BUILD.bazel",
            "rust_library(name = \"model\", srcs = glob([\"src/*.rs\"]))",
        ),
        source(
            "build_system/model/src/lib.rs",
            "#[path = OTHER] mod other;\n#[path = \"missing.rs\"] mod missing;\n\
             #[other = \"value.rs\"] mod value;\n",
        ),
    ]);
    assert!(violations.is_empty(), "{}", messages(&violations));
}

#[test]
fn core_path_attributes_cannot_cross_component_packages() {
    let violations = inspect(vec![
        source(
            "core/crates/facade/BUILD.bazel",
            "rust_library(name = \"facade\", srcs = [\"src/lib.rs\"])",
        ),
        source(
            "core/crates/facade/src/lib.rs",
            "#[path = \"../../model/src/value.rs\"] mod value;",
        ),
        source(
            "core/crates/model/BUILD.bazel",
            "rust_library(name = \"model\", srcs = [\"src/value.rs\"])",
        ),
        source("core/crates/model/src/value.rs", "pub struct Value;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("#[path] cannot cross"), "{report}");
}
