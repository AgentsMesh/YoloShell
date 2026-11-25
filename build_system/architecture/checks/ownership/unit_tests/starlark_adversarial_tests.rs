use super::support::{inspect, messages, source};
use crate::RUNTIME_SOURCES_TARGET;

#[test]
fn comments_and_string_literals_cannot_forge_an_owner() {
    let build = r#"
# rust_library(name = "forged", srcs = ["src/lib.rs"])
NOTICE = "rust_library(name = 'forged', srcs = ['src/lib.rs'])"
"#;
    let violations = inspect(vec![
        source("core/crates/model/BUILD.bazel", build),
        source("core/crates/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn cross_package_filegroup_aggregation_and_facades_are_rejected() {
    let facade = format!(
        "filegroup(name = \"component\", \
         srcs = [\"//build_system/model:{RUNTIME_SOURCES_TARGET}\"])\n\
         rust_library(name = \"facade\", srcs = [\":component\"])"
    );
    let component =
        format!("filegroup(name = \"{RUNTIME_SOURCES_TARGET}\", srcs = glob([\"src/*.rs\"]))");
    let violations = inspect(vec![
        source("build_system/facade/BUILD.bazel", facade),
        source("build_system/model/BUILD.bazel", component),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(
        report.contains("cannot cross package boundaries"),
        "{report}"
    );
    assert!(report.contains("root facade cannot compile"), "{report}");
}

#[test]
fn opaque_rule_wrappers_cannot_forge_a_real_rust_owner() {
    let build = r#"
load(":wrapper.bzl", "component")
component(name = "model", srcs = ["src/value.rs"])
"#;
    let wrapper = r#"
def component(name, srcs):
    rust_library(name = name, srcs = srcs)
"#;
    let violations = inspect(vec![
        source("build_system/model/BUILD.bazel", build),
        source("build_system/model/wrapper.bzl", wrapper),
        source("build_system/model/src/value.rs", "pub struct Value;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn starlark_macros_are_not_real_production_owners() {
    let wrapper = format!(
        "rust_library(name = \"facade\", \
         srcs = [\"//build_system/model:{RUNTIME_SOURCES_TARGET}\"])"
    );
    let component =
        format!("filegroup(name = \"{RUNTIME_SOURCES_TARGET}\", srcs = [\"src/lib.rs\"])");
    let violations = inspect(vec![
        source("build_system/facade/wrapper.bzl", wrapper),
        source("build_system/model/BUILD.bazel", component),
        source("build_system/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(
        report.contains(crate::constants::MISSING_OWNER_MESSAGE),
        "{report}"
    );
}

#[test]
fn core_facades_cannot_aggregate_component_implementation() {
    let facade = format!(
        "rust_library(name = \"facade\", \
         srcs = [\"//core/crates/model:{RUNTIME_SOURCES_TARGET}\"])"
    );
    let component =
        format!("filegroup(name = \"{RUNTIME_SOURCES_TARGET}\", srcs = [\"src/lib.rs\"])");
    let violations = inspect(vec![
        source("core/BUILD.bazel", facade),
        source("core/crates/model/BUILD.bazel", component),
        source("core/crates/model/src/lib.rs", "pub struct Model;"),
    ]);
    let report = messages(&violations);
    assert!(report.contains("root facade cannot compile"), "{report}");
    assert!(
        report.contains("cannot cross package boundaries"),
        "{report}"
    );
}
