use crate::constants::{
    RETIRED_FRONTEND_DIRECTORY_PARTS, RETIRED_FRONTEND_FILE_PARTS, RETIRED_SCRIPT_EXTENSIONS,
    joined,
};
use yolo_architecture_cargo_policy_test_support::metadata_manifest;
use yolo_architecture_test_support::Fixture;

fn violations(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_empty_script_sources_by_extension() {
    let fixture = Fixture::new();
    for (index, extension) in RETIRED_SCRIPT_EXTENSIONS.iter().enumerate() {
        fixture.write(&format!("legacy/source-{index}.{extension}"), "");
    }
    let result = violations(&fixture);
    assert_eq!(result.len(), RETIRED_SCRIPT_EXTENSIONS.len());
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("tooling is retired"))
    );
}

#[test]
fn rejects_frontend_manifests_even_without_dependency_terms() {
    let fixture = Fixture::new();
    for parts in RETIRED_FRONTEND_FILE_PARTS {
        fixture.write(&joined(parts), "{}\n");
    }
    let result = violations(&fixture);
    let retired = result
        .iter()
        .filter(|entry| entry.contains("tooling is retired"))
        .count();
    assert_eq!(retired, RETIRED_FRONTEND_FILE_PARTS.len());
}

#[test]
fn rejects_empty_frontend_tool_directories_without_scanning_them() {
    let fixture = Fixture::new();
    for parts in RETIRED_FRONTEND_DIRECTORY_PARTS {
        std::fs::create_dir_all(fixture.root().join(joined(parts))).unwrap();
    }
    let result = violations(&fixture);
    assert_eq!(result.len(), RETIRED_FRONTEND_DIRECTORY_PARTS.len());
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("tooling is retired"))
    );
}

#[test]
fn rejects_native_sources_that_launch_a_retired_runtime() {
    let fixture = Fixture::new();
    let runtime = joined(&["no", "de"]);
    fixture.write(
        "core/src/direct.rs",
        format!("std::process::Command::new(\"{runtime}\").spawn();\n"),
    );
    fixture.write(
        "core/src/generated_shell.rs",
        format!("let script = \"#!/bin/sh\\nexec {runtime} tool\\n\";\n"),
    );
    fixture.write("build_system/legacy.sh", format!("exec {runtime} tool\n"));
    let result = violations(&fixture);
    assert_eq!(result.len(), 3);
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("disallowed frontend dependency or tool"))
    );
}

#[test]
fn rejects_retired_cross_platform_ui_outside_rust_manifests() {
    let fixture = Fixture::new();
    let first = joined(&["wg", "pu"]);
    let second = joined(&["e", "gui"]);
    fixture.write(
        "MODULE.bazel",
        format!("bazel_dep(name = \"{first}\")\n# {second}\n"),
    );
    fixture.write(&format!("vendor/{first}/README.md"), "");
    let result = violations(&fixture);
    assert_eq!(result.len(), 3);
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("retired cross-platform UI technology"))
    );
}

#[test]
fn distinguishes_process_tree_nodes_from_the_retired_runtime() {
    let fixture = Fixture::new();
    let runtime = joined(&["no", "de"]);
    fixture.write(&format!("core/src/{runtime}.rs"), "pub struct HubNode;\n");
    fixture.write(
        "docs/tree.md",
        format!("Each {runtime} in the process tree retains an identity.\n"),
    );
    assert!(violations(&fixture).is_empty());

    fixture.write(
        "docs/runtime.md",
        format!("The retired {runtime}.js runtime must remain absent.\n"),
    );
    let result = violations(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("disallowed frontend dependency or tool"));
}

#[test]
fn permits_native_manifests_and_data_files() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", metadata_manifest(""));
    fixture.write("clients/tui/descriptor/fixtures/native-profile.json", "{}\n");
    assert!(violations(&fixture).is_empty());
}
