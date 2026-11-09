mod communication_tests;
mod component_deps_tests;
mod component_layout_tests;
mod provider_policy_tests;

use std::path::PathBuf;
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_repository::{
    constants::{BUILD_SYSTEM_DIRECTORY, INTEGRATION_TESTS_DIRECTORY, THIRD_PARTY_DIRECTORY},
    repository_rules,
};

fn source(path: &str) -> SourceFile {
    SourceFile {
        absolute: PathBuf::from(path),
        relative: path.to_owned(),
        text: String::new(),
    }
}

fn owned_path(directory: &str, path: &str) -> String {
    format!("{directory}/{path}")
}

fn violations(files: &[SourceFile], directories: &[&str]) -> Vec<Violation> {
    let directories = directories
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    repository_rules::inspect(files, &directories, &mut result);
    result
}

#[test]
fn retired_top_level_directories_are_rejected() {
    let result = violations(&[], &["rust", "tools", "tests"]);
    assert_eq!(result.len(), 3);
    assert!(
        result
            .iter()
            .all(|item| item.message.contains("domain ownership"))
    );
}

#[test]
fn retired_flat_agent_roots_and_children_are_rejected() {
    let directories = [
        "core/crates/agent_contract",
        "core/crates/agent_runtime/src",
        "core/crates/agent_system/claude",
    ];
    let result = violations(&[], &directories);
    assert_eq!(result.len(), directories.len());
    assert!(
        result
            .iter()
            .all(|item| item.message.contains("flat Agent"))
    );
}

#[test]
fn first_party_starlark_is_owned_by_the_build_system() {
    let files = [source("core/rules.bzl"), source("clients/tui/rules.bzl")];
    let result = violations(&files, &[]);
    assert_eq!(result.len(), files.len());
    assert!(
        result
            .iter()
            .all(|item| item.message.contains(BUILD_SYSTEM_DIRECTORY))
    );
}

#[test]
fn domain_layout_and_owned_starlark_are_admitted() {
    let files = [
        source(&owned_path(BUILD_SYSTEM_DIRECTORY, "rust/rules.bzl")),
        source(&owned_path(THIRD_PARTY_DIRECTORY, "vendor/rules.bzl")),
    ];
    assert!(violations(&files, &["core", INTEGRATION_TESTS_DIRECTORY, "tui"]).is_empty());
}

#[test]
fn policy_file_discovery_keeps_retired_build_inputs_visible() {
    for path in [
        "core/go.mod",
        "core/helper.go",
        "Package.swift",
        "BUILD",
        "WORKSPACE",
    ] {
        assert!(
            repository_rules::is_policy_file(std::path::Path::new(path)),
            "{path}"
        );
    }
    assert!(!repository_rules::is_policy_file(std::path::Path::new(
        "core/source.rs"
    )));
}

#[test]
fn retired_go_tool_directories_are_rejected_independent_of_file_contents() {
    for term in [["rules", "_go"].concat(), ["ga", "zelle"].concat()] {
        let directories = [
            format!("build_system/{term}"),
            format!("build_system/{term}.cache"),
        ];
        for directory in directories {
            let result = violations(&[], &[&directory]);
            assert_eq!(result.len(), 1);
            assert!(result[0].message.contains("Go tooling is retired"));
        }
        let mut file = source("WORKSPACE");
        file.text = term;
        assert!(
            violations(&[file], &[])
                .iter()
                .any(|violation| violation.message.contains("Go Bazel rules"))
        );
    }
}
