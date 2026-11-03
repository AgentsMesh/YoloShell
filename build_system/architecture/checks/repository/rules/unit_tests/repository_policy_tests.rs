use yolo_architecture_foundation::{model::Violation, text::joined};
use yolo_architecture_repository::repository_rules;
use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    let files = yolo_architecture_test_support::load(fixture.root()).unwrap();
    let directories = yolo_architecture_test_support::directories(fixture.root()).unwrap();
    let mut violations = Vec::new();
    repository_rules::inspect(&files, &directories, &mut violations);
    violations
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

fn directory_messages(directories: &[&str]) -> Vec<String> {
    let directories = directories
        .iter()
        .map(|directory| (*directory).to_owned())
        .collect::<Vec<_>>();
    let mut violations = Vec::<Violation>::new();
    repository_rules::inspect(&[], &directories, &mut violations);
    violations
        .into_iter()
        .map(|item| item.to_string())
        .collect()
}

#[test]
fn rejects_go_sources_and_module_manifests() {
    let fixture = Fixture::new();
    for path in ["cmd/helper.go", "go.mod", "go.sum", "go.work"] {
        fixture.write(path, "");
    }
    let result = messages(&fixture);
    assert_eq!(result.len(), 4);
    assert!(result.iter().all(|entry| entry.contains("Go sources")));
}

#[test]
fn rejects_go_rules_and_generators_in_bazel_metadata() {
    let fixture = Fixture::new();
    let rules = joined(&["rules", "_go"]);
    let generator = joined(&["ga", "zelle"]);
    fixture.write("MODULE.bazel", format!("bazel_dep(name = \"{rules}\")\n"));
    fixture.write(
        "native/BUILD.bazel",
        format!("load(\"@{generator}//:defs.bzl\", \"target\")\n"),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|entry| entry.contains("Go Bazel")));
}

#[test]
fn rejects_secondary_application_build_entries() {
    let fixture = Fixture::new();
    let manifests = [
        "Package.swift",
        "Makefile",
        "Justfile",
        "CMakeLists.txt",
        "meson.build",
        "build.gradle.kts",
        "project.yml",
    ];
    for path in manifests {
        fixture.write(path, "");
    }
    let result = messages(&fixture);
    assert_eq!(result.len(), manifests.len());
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("only application build entry"))
    );
}

#[test]
fn rejects_all_lowercase_tests_directories() {
    let result = directory_messages(&[
        "core/crates/example/tests",
        "build_system/example/tests/fixtures",
    ]);
    assert_eq!(result.len(), 2);
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("unit_tests or integration_tests"))
    );
}

#[test]
fn admits_explicit_unit_integration_and_support_test_layouts() {
    let result = directory_messages(&[
        "core/crates/example/unit_tests",
        "core/crates/example/integration_tests",
        "core/crates/example/test_support",
        "integration_tests/runtime",
        "integration_tests/runtime/resilience/support",
    ]);
    assert!(result.is_empty(), "{result:?}");
}

#[test]
fn rejects_retired_top_level_implementation_and_tooling_directories() {
    let result = directory_messages(&["rust", "rust/crates", "tools", "tests"]);
    assert_eq!(result.len(), 4, "{result:?}");
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("domain ownership"))
    );
}

#[test]
fn requires_first_party_bazel_extensions_below_build_system() {
    let fixture = Fixture::new();
    for path in [
        "core/rules.bzl",
        "integration_tests/rules.bzl",
        "plugins/tui/rules.bzl",
    ] {
        fixture.write(path, "def target():\n    pass\n");
    }
    fixture.write(
        "build_system/bazel/rust/rules.bzl",
        "def target():\n    pass\n",
    );
    fixture.write("third_party/vendor/rules.bzl", "def target():\n    pass\n");

    let result = messages(&fixture);
    assert_eq!(result.len(), 3, "{result:?}");
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("below build_system"))
    );
}

#[test]
fn rejects_module_scopes_above_the_direct_line_budget() {
    let fixture = Fixture::new();
    let mut composition = String::new();
    for index in 0..9 {
        fixture.write(
            &format!("core/crates/example/src/generated/fragment_{index}.rs"),
            body(),
        );
        composition.push_str(&format!("include!(\"generated/fragment_{index}.rs\");\n"));
    }
    fixture.write("core/crates/example/src/mod.rs", composition);

    let result = messages(&fixture);
    assert_eq!(result.len(), 1, "{result:?}");
    assert!(result[0].contains("module scope exceeds"), "{result:?}");
    assert!(result[0].contains("core/crates/example/src/mod.rs"));
}

#[test]
fn admits_nested_modules_scoped_into_their_own_children() {
    let fixture = Fixture::new();
    let mut composition = String::new();
    for index in 0..9 {
        fixture.write(&format!("core/crates/example/src/leaf_{index}.rs"), body());
        composition.push_str(&format!("mod leaf_{index};\n"));
    }
    fixture.write("core/crates/example/src/mod.rs", composition);

    let result = messages(&fixture);
    assert!(
        !result.iter().any(|entry| entry.contains("module scope")),
        "{result:?}"
    );
}

fn body() -> String {
    "fn placeholder() {}\n".repeat(150)
}
