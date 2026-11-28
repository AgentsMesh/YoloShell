use super::support::{inspect, messages, source};
use crate::{
    MAX_BUILD_SYSTEM_PRODUCTION_LINES_PER_PACKAGE, MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_PACKAGE,
    MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_TARGET, MAX_CORE_PRODUCTION_LINES_PER_PACKAGE,
    MAX_CORE_PRODUCTION_SOURCES_PER_PACKAGE,
};

const PACKAGE: &str = "build_system/example";

#[test]
fn build_system_target_source_budget_is_enforced() {
    let source_count = MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_TARGET + 1;
    let names = (0..source_count)
        .map(|index| format!("\"src/source_{index}.rs\""))
        .collect::<Vec<_>>()
        .join(",");
    let build = format!("rust_library(name = \"library\", srcs = [{names}])");
    let mut files = vec![source(&format!("{PACKAGE}/BUILD.bazel"), build)];
    files.extend((0..source_count).map(|index| {
        source(
            &format!("{PACKAGE}/src/source_{index}.rs"),
            "pub struct Value;",
        )
    }));
    let violations = inspect(files);
    let report = messages(&violations);
    assert!(report.contains("production source budget"), "{report}");
}

#[test]
fn package_line_budget_is_enforced_independently_of_source_count() {
    let lines = (0..=MAX_BUILD_SYSTEM_PRODUCTION_LINES_PER_PACKAGE)
        .map(|_| "pub struct Value;")
        .collect::<Vec<_>>()
        .join("\n");
    let files = vec![
        source(
            &format!("{PACKAGE}/BUILD.bazel"),
            "rust_library(name = \"library\", srcs = [\"src/lib.rs\"])",
        ),
        source(&format!("{PACKAGE}/src/lib.rs"), lines),
    ];
    let report = messages(&inspect(files));
    assert!(report.contains("production Rust line budget"), "{report}");
}

#[test]
fn build_system_root_rust_sources_require_direct_owners() {
    let report = messages(&inspect(vec![
        source("build_system/BUILD.bazel", "filegroup(name = \"data\")"),
        source("build_system/orphan.rs", "pub struct Orphan;"),
    ]));
    assert!(report.contains("must belong to exactly one"), "{report}");
}

#[test]
fn multiple_targets_cannot_hide_an_oversized_package() {
    let source_count = MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_PACKAGE + 1;
    let split = source_count / 2;
    let names = |range: std::ops::Range<usize>| {
        range
            .map(|index| format!("\"src/source_{index}.rs\""))
            .collect::<Vec<_>>()
            .join(",")
    };
    let build = format!(
        concat!(
            "rust_library(name = \"first\", srcs = [{}])\n",
            "rust_library(name = \"second\", srcs = [{}])\n",
        ),
        names(0..split),
        names(split..source_count),
    );
    let mut files = vec![source(&format!("{PACKAGE}/BUILD.bazel"), build)];
    files.extend((0..source_count).map(|index| {
        source(
            &format!("{PACKAGE}/src/source_{index}.rs"),
            "pub struct Value;",
        )
    }));
    let report = messages(&inspect(files));
    assert!(report.contains("Bazel package exceeds"), "{report}");
    assert!(!report.contains("target exceeds"), "{report}");
}

#[test]
fn core_package_source_budget_is_enforced() {
    let report = messages(&inspect(package_fixture(
        "core/crates/example",
        MAX_CORE_PRODUCTION_SOURCES_PER_PACKAGE + 1,
    )));
    assert!(
        report.contains("core or client Bazel package exceeds the production Rust source budget"),
        "{report}"
    );
}

#[test]
fn core_package_line_budget_is_enforced() {
    let lines = "pub struct Value;\n".repeat(MAX_CORE_PRODUCTION_LINES_PER_PACKAGE + 1);
    let files = vec![
        source(
            "core/crates/example/BUILD.bazel",
            "rust_library(name = \"library\", srcs = [\"src/lib.rs\"])",
        ),
        source("core/crates/example/src/lib.rs", lines),
    ];
    let report = messages(&inspect(files));
    assert!(
        report.contains("core or client Bazel package exceeds the production Rust line budget"),
        "{report}"
    );
}

#[test]
fn core_package_baseline_admits_the_recorded_size_and_rejects_growth() {
    let (package, sources, _) = crate::constants::PACKAGE_BASELINE
        .iter()
        .find(|entry| entry.0 == "core/crates/agent/providers/claude")
        .copied()
        .unwrap();
    let admitted = messages(&inspect(package_fixture(package, sources)));
    assert!(!admitted.contains("core or client Bazel package"), "{admitted}");
    let report = messages(&inspect(package_fixture(package, sources + 1)));
    assert!(report.contains(&format!("{} > {sources}", sources + 1)), "{report}");
}

fn package_fixture(package: &str, source_count: usize) -> Vec<yolo_architecture_foundation::SourceFile> {
    let names = (0..source_count)
        .map(|index| format!("\"src/source_{index}.rs\""))
        .collect::<Vec<_>>()
        .join(",");
    let build = format!("rust_library(name = \"library\", srcs = [{names}])");
    let mut files = vec![source(&format!("{package}/BUILD.bazel"), build)];
    files.extend((0..source_count).map(|index| {
        source(
            &format!("{package}/src/source_{index}.rs"),
            "pub struct Value;",
        )
    }));
    files
}
