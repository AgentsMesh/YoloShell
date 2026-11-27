use yolo_architecture_cargo_policy_test_support::metadata_manifest;
use yolo_architecture_repository::constants::INTEGRATION_TESTS_DIRECTORY;
use yolo_architecture_test_support::Fixture;

const VALID_COVERAGE_CONFIGURATION: &str = concat!(
    "coverage:rust_tooling --config=rust\n",
    "coverage:rust_tooling --instrumentation_filter=^//(core|plugins|build_system|integration_tests)[/:]\n",
    "coverage:rust --instrumentation_filter=^//(core|plugins|integration_tests)[/:]\n",
    "coverage:rust --@rules_rust//rust/settings:extra_rustc_env=RUSTC_BOOTSTRAP=1\n",
    "coverage:rust --@rules_rust//rust/settings:extra_rustc_flag=-Zcoverage-options=branch\n",
);
const COVERAGE_CONFIGURATION_VIOLATION: &str = "Rust coverage configuration";

fn messages(cargo: &str, module: &str) -> Vec<String> {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", metadata_manifest(cargo));
    fixture.write("MODULE.bazel", module);
    fixture.write(".bazelrc", VALID_COVERAGE_CONFIGURATION);
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn accepts_exact_core_coverage_configuration() {
    assert!(messages("rust-version = \"1.93.1\"\n", &module("\"1.93.1\"")).is_empty());
}

#[test]
fn rejects_stale_product_roots_and_mutated_rules_rust_labels() {
    let fixture = Fixture::new();
    fixture.write(
        ".bazelrc",
        concat!(
            "coverage:rust --instrumentation_filter=^//rust[/:]\n",
            "coverage:rust --@rules_rust//core/settings:extra_rustc_env=RUSTC_BOOTSTRAP=1\n",
            "coverage:rust --@rules_rust//core/settings:extra_rustc_flag=-Zcoverage-options=branch\n",
        ),
    );
    let result = crate::inspect(fixture.root()).unwrap();
    assert_eq!(result.len(), crate::RUST_COVERAGE_DECLARATIONS.len());
    assert!(result.iter().all(|violation| {
        violation
            .to_string()
            .contains(COVERAGE_CONFIGURATION_VIOLATION)
    }));
}

fn module(versions: &str) -> String {
    format!("rust.toolchain(\n    versions = [{versions}],\n)\n")
}

#[test]
fn rejects_tooling_filter_that_drops_transitive_core_source_inventory() {
    let fixture = Fixture::new();
    fixture.write(
        ".bazelrc",
        VALID_COVERAGE_CONFIGURATION.replace(
            "coverage:rust_tooling --instrumentation_filter=^//(core|plugins|build_system|integration_tests)[/:]",
            "coverage:rust_tooling --instrumentation_filter=^//(build_system|integration_tests)[/:]",
        ),
    );
    let result = crate::inspect(fixture.root()).unwrap();
    assert_eq!(result.len(), 1);
    assert!(
        result[0]
            .to_string()
            .contains(COVERAGE_CONFIGURATION_VIOLATION)
    );
}

#[test]
fn rejects_filters_that_omit_repository_test_sources_or_production_roots() {
    for (original, incomplete) in [
        ("(core|plugins|integration_tests)", "core"),
        (
            "(core|plugins|integration_tests)",
            INTEGRATION_TESTS_DIRECTORY,
        ),
        (
            "(core|plugins|integration_tests)",
            "(core|integration_tests)",
        ),
        (
            "(core|plugins|build_system|integration_tests)",
            "(core|build_system)",
        ),
        (
            "(core|plugins|build_system|integration_tests)",
            "(core|integration_tests)",
        ),
        (
            "(core|plugins|build_system|integration_tests)",
            "(core|build_system|integration_tests)",
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(
            ".bazelrc",
            VALID_COVERAGE_CONFIGURATION.replace(original, incomplete),
        );
        let violations = crate::inspect(fixture.root()).unwrap();
        assert_eq!(violations.len(), 1);
        assert!(
            violations[0]
                .to_string()
                .contains(COVERAGE_CONFIGURATION_VIOLATION)
        );
    }
}

#[test]
fn accepts_matching_pinned_versions() {
    assert!(messages("rust-version = \"1.93.1\"\n", &module("\"1.93.1\"")).is_empty());
}

#[test]
fn rejects_a_cargo_and_bazel_mismatch() {
    let result = messages("rust-version = \"1.85.0\"\n", &module("\"1.93.1\""));
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("differs from Bazel toolchain"));
}

#[test]
fn rejects_missing_unpinned_and_multiple_versions() {
    let cases = [
        ("", module("\"1.93.1\"")),
        ("rust-version = \"1.93\"\n", module("\"1.93.1\"")),
        (
            "rust-version = \"1.93.1\"\n",
            module("\"1.93.1\", \"1.92.0\""),
        ),
        ("rust-version = \"1.93.1\"\n", module("")),
    ];
    for (cargo, module) in cases {
        let result = messages(cargo, &module);
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(result[0].contains("exactly one major.minor.patch"));
    }
}

#[test]
fn reads_multiline_bazel_versions_and_ignores_unrelated_lists() {
    let module = concat!(
        "other(versions = [\"0.1.0\"])\n",
        "rust.toolchain(\n",
        "    edition = \"2024\",\n",
        "    versions = [\n",
        "        \"1.93.1\",\n",
        "    ],\n",
        ")\n",
    );
    assert!(messages("rust-version = \"1.93.1\"\n", module).is_empty());
}

#[test]
fn skips_partial_fixtures_without_both_manifests() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", metadata_manifest("rust-version = \"1.85\"\n"));
    assert!(crate::inspect(fixture.root()).unwrap().is_empty());
}
