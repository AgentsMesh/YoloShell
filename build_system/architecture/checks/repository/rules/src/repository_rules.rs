use crate::constants::{
    BUILD_SYSTEM_DIRECTORY, RETIRED_AGENT_DIRECTORIES, RETIRED_TOP_LEVEL_DIRECTORIES,
    THIRD_PARTY_DIRECTORY,
};
use std::path::Path;
use yolo_architecture_foundation::{
    SourceFile,
    model::Violation,
    text::{contains_ascii, first_line_ascii, joined},
};

const GO_MANIFESTS: &[&str] = &["go.mod", "go.sum", "go.work"];
const SECONDARY_BUILD_FILES: &[&str] = &[
    "Package.swift",
    "Makefile",
    "GNUmakefile",
    "Justfile",
    "CMakeLists.txt",
    "meson.build",
    "build.gradle",
    "build.gradle.kts",
    "Podfile",
    "Cartfile",
    "project.yml",
    "Tuist.swift",
];
const SECONDARY_BUILD_DIRECTORIES: &[&str] = &[".xcodeproj", ".xcworkspace"];
const BAZEL_FILE_NAMES: &[&str] = &["BUILD", "WORKSPACE", "WORKSPACE.bazel"];
const STARLARK_EXTENSIONS: &[&str] = &["bazel", "bzl"];
const GO_BUILD_TERM_PARTS: &[&[&str]] = &[&["rules", "_", "go"], &["ga", "zelle"]];
const LOWERCASE_TEST_DIRECTORY: &str = "tests";
const STARLARK_EXTENSION: &str = "bzl";
const TEST_DIRECTORY_POLICY_MESSAGE: &str =
    "lowercase tests directories must be split into unit_tests or integration_tests";
const RETIRED_TOP_LEVEL_POLICY_MESSAGE: &str =
    "top-level implementation and tooling directories must use domain ownership";
const RETIRED_AGENT_LAYOUT_POLICY_MESSAGE: &str =
    "flat Agent component roots are retired; use core/crates/agent/{contract,runtime,system}";
const STARLARK_OWNERSHIP_POLICY_MESSAGE: &str =
    "first-party Bazel extensions must live below build_system";

pub fn is_policy_file(path: &Path) -> bool {
    let name = file_name(path);
    go_artifact(path, name)
        || SECONDARY_BUILD_FILES.contains(&name)
        || BAZEL_FILE_NAMES.contains(&name)
}

pub fn inspect(files: &[SourceFile], directories: &[String], violations: &mut Vec<Violation>) {
    crate::component_deps::inspect(files, violations);
    crate::directory_budget::inspect(files, violations);
    crate::include_allowlist::inspect(files, violations);
    crate::module_closure::inspect(files, violations);
    crate::provider_policy::inspect(files, violations);
    for file in files {
        inspect_file(file, violations);
    }
    for directory in directories {
        if forbidden_build_directory(directory) {
            violations.push(Violation::file(
                directory,
                "Bazel is the only application build entry point",
            ));
        } else if go_tool_path(directory) {
            violations.push(Violation::file(
                directory,
                "Go tooling is retired from the native repository",
            ));
        } else if retired_agent_directory(directory) {
            violations.push(Violation::file(
                directory,
                RETIRED_AGENT_LAYOUT_POLICY_MESSAGE,
            ));
        } else if retired_top_level_directory(directory) {
            violations.push(Violation::file(directory, RETIRED_TOP_LEVEL_POLICY_MESSAGE));
        } else if lowercase_test_directory(directory) {
            violations.push(Violation::file(directory, TEST_DIRECTORY_POLICY_MESSAGE));
        }
    }
}

fn inspect_file(file: &SourceFile, violations: &mut Vec<Violation>) {
    crate::communication::inspect(file, violations);
    crate::component_layout::inspect(file, violations);
    let name = file_name(&file.absolute);
    if go_artifact(&file.absolute, name) {
        violations.push(Violation::file(
            &file.relative,
            "Go sources and manifests are retired from the native repository",
        ));
    }
    if secondary_build_file(name) {
        violations.push(Violation::file(
            &file.relative,
            "Bazel is the only application build entry point",
        ));
    }
    if bazel_metadata(&file.relative) {
        inspect_go_build_terms(file, violations);
    }
    if misplaced_starlark_extension(&file.relative) {
        violations.push(Violation::file(
            &file.relative,
            STARLARK_OWNERSHIP_POLICY_MESSAGE,
        ));
    }
}

fn inspect_go_build_terms(file: &SourceFile, violations: &mut Vec<Violation>) {
    for parts in GO_BUILD_TERM_PARTS {
        let term = joined(parts);
        if let Some(line) = first_line_ascii(&file.text, &term) {
            violations.push(Violation::line(
                &file.relative,
                line,
                "Go Bazel rules and generators are retired from the native repository",
            ));
        }
    }
}

fn go_artifact(path: &Path, name: &str) -> bool {
    GO_MANIFESTS.contains(&name)
        || path.extension().and_then(|value| value.to_str()) == Some("go")
        || go_tool_path(&path.to_string_lossy())
}

fn secondary_build_file(name: &str) -> bool {
    SECONDARY_BUILD_FILES.contains(&name)
}

fn forbidden_build_directory(relative: &str) -> bool {
    let lower = relative.to_ascii_lowercase();
    SECONDARY_BUILD_DIRECTORIES
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn lowercase_test_directory(relative: &str) -> bool {
    Path::new(relative)
        .components()
        .any(|component| component.as_os_str() == LOWERCASE_TEST_DIRECTORY)
}

fn retired_top_level_directory(relative: &str) -> bool {
    relative
        .split('/')
        .next()
        .is_some_and(|directory| RETIRED_TOP_LEVEL_DIRECTORIES.contains(&directory))
}

fn retired_agent_directory(relative: &str) -> bool {
    RETIRED_AGENT_DIRECTORIES.iter().any(|root| {
        relative == *root
            || relative
                .strip_prefix(root)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn misplaced_starlark_extension(relative: &str) -> bool {
    let path = Path::new(relative);
    path.extension().and_then(|value| value.to_str()) == Some(STARLARK_EXTENSION)
        && !relative.starts_with(&format!("{BUILD_SYSTEM_DIRECTORY}/"))
        && !relative.starts_with(&format!("{THIRD_PARTY_DIRECTORY}/"))
}

fn go_tool_path(relative: &str) -> bool {
    GO_BUILD_TERM_PARTS.iter().any(|parts| {
        let term = joined(parts);
        relative.split('/').any(|component| {
            component.eq_ignore_ascii_case(&term) || contains_ascii(component, &format!("{term}."))
        })
    })
}

fn bazel_metadata(relative: &str) -> bool {
    let path = Path::new(relative);
    let name = file_name(path);
    BAZEL_FILE_NAMES.contains(&name)
        || path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| STARLARK_EXTENSIONS.contains(&extension))
}

fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
}
