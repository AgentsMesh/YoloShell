use std::path::PathBuf;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_repository::repository_rules;

const DEPS: &str = "build_system/bazel/rust/component_deps.bzl";
const FACADE: &str = "core/src/lib.rs";
const TOPOLOGY: &str = "build_system/architecture/topology.bzl";
const PROVIDERS: &str = "build_system/bazel/rust/providers.bzl";

fn source(path: &str, text: &str) -> SourceFile {
    SourceFile {
        absolute: PathBuf::from(path),
        relative: path.to_owned(),
        text: text.to_owned(),
    }
}

fn messages(files: &[SourceFile]) -> Vec<String> {
    let mut violations = Vec::new();
    repository_rules::inspect(files, &[], &mut violations);
    violations
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

fn report(deps: &str, facade: Option<&str>) -> Vec<String> {
    let mut files = vec![source(DEPS, deps)];
    if let Some(facade) = facade {
        files.push(source(FACADE, facade));
    }
    messages(&files)
}

fn assert_reports(report: &[String], needle: &str) {
    assert!(
        report.iter().any(|message| message.contains(needle)),
        "{report:?}"
    );
}

#[test]
fn admits_a_referenced_string_dependency() {
    let report = report(
        "RUNTIME_COMPONENT_DEPS = [\"//core/crates/x:yolo_x\"]\n",
        Some("pub use yolo_x as x;\n"),
    );

    assert!(report.is_empty(), "{report:?}");
}

#[test]
fn rejects_a_dependency_missing_from_the_facade() {
    let report = report(
        "RUNTIME_COMPONENT_DEPS = [\"//core/crates/x:yolo_x\"]\n",
        None,
    );

    assert_reports(&report, "yolo_x");
}

#[test]
fn resolves_a_named_component() {
    let deps = "X = \"//core/crates/x:yolo_x\"\nRUNTIME_COMPONENT_DEPS = [X]\n";
    let report = report(deps, Some("pub use yolo_x as x;\n"));

    assert!(report.is_empty(), "{report:?}");
}

#[test]
fn rejects_an_unresolved_name() {
    let report = report(
        "RUNTIME_COMPONENT_DEPS = [MISSING]\n",
        Some("pub use yolo_x as x;\n"),
    );

    assert_reports(&report, "no string assignment: MISSING");
}

#[test]
fn rejects_a_comprehension_as_the_dependency_list() {
    let report = report(
        "RUNTIME_COMPONENT_DEPS = [item for item in OTHER]\n",
        Some("pub use yolo_x as x;\n"),
    );

    assert_reports(&report, "must be a literal list");
}

#[test]
fn matches_whole_identifiers_only() {
    let report = report(
        "RUNTIME_COMPONENT_DEPS = [\"//core/crates/x:yolo_x\"]\n",
        Some("pub use yolo_x_helper as helper;\n"),
    );

    assert_reports(&report, "yolo_x");
}

#[test]
fn ignores_a_commented_dependency() {
    let deps = "RUNTIME_COMPONENT_DEPS = [\n    \"//core/crates/x:yolo_x\",\n    # \"//core/crates/gone:yolo_gone\",\n]\n";
    let report = report(deps, Some("pub use yolo_x as x;\n"));

    assert!(report.is_empty(), "{report:?}");
}

#[test]
fn resolves_topology_constants_and_local_aliases() {
    let deps = "load(\"//build_system/architecture:topology.bzl\", \"YOLO_OTHER\")\n\
                ALIAS = YOLO_OTHER\n\
                RUNTIME_COMPONENT_DEPS = [ALIAS]\n";
    let topology = "YOLO_OTHER = \"//core/crates/other:yolo_other\"\n";
    let mut files = vec![source(DEPS, deps), source(TOPOLOGY, topology)];

    let result = messages(&files);
    assert_eq!(result.len(), 1, "{result:?}");
    assert!(result[0].contains("yolo_other"), "{result:?}");
    assert!(!result[0].contains("no string assignment"), "{result:?}");

    files.push(source(FACADE, "pub use yolo_other as other;\n"));
    assert!(messages(&files).is_empty());
}

fn provider_report(providers: &str, facade: &str) -> Vec<String> {
    let files = vec![
        source(
            DEPS,
            "RUNTIME_COMPONENT_DEPS = [\"//core/crates/x:yolo_x\"]\n",
        ),
        source(PROVIDERS, providers),
        source(FACADE, facade),
    ];
    messages(&files)
}

#[test]
fn rejects_a_provider_runtime_component_missing_from_the_facade() {
    let providers = "PROVIDERS = [\n    struct(\n        runtime_components = [\n            \"//core/crates/provider:yolo_provider\",\n        ],\n    ),\n]\n";
    let report = provider_report(providers, "pub use yolo_x as x;\n");

    assert_reports(&report, "yolo_provider");
}

#[test]
fn admits_a_provider_runtime_component_referenced_by_the_facade() {
    let providers = "PROVIDERS = [\n    struct(\n        runtime_components = [\n            \"//core/crates/provider:yolo_provider\",\n        ],\n    ),\n]\n";
    let report = provider_report(
        providers,
        "pub use yolo_x as x;\npub use yolo_provider as provider;\n",
    );

    assert!(report.is_empty(), "{report:?}");
}

#[test]
fn ignores_non_core_entries_in_the_provider_table() {
    let providers = "PROVIDERS = [\n    struct(\n        runtime_components = [\n            \"//clients/tui:unit_tests\",\n        ],\n    ),\n]\n";
    let report = provider_report(providers, "pub use yolo_x as x;\n");

    assert!(report.is_empty(), "{report:?}");
}
