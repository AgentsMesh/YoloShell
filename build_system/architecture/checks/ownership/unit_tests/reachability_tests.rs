use super::support::source;
use crate::constants::{
    UNCERTAIN_TEST_MODULE_MESSAGE, UNREACHABLE_TEST_MODULE_MESSAGE, UNRESOLVED_TEST_ROOT_MESSAGE,
};
use yolo_architecture_foundation::{SourceFile, model::Violation};

const PACKAGE: &str = "core/crates/example";
const BUILD: &str = "core/crates/example/BUILD.bazel";
const ROOT: &str = "core/crates/example/src/lib.rs";
const CASE: &str = "core/crates/example/unit_tests/case.rs";
const TEST_BODY: &str = "#[test] fn scenario() {}";
const BASE_TARGET: &str =
    "rust_test(name = \"unit\", crate_root = \"src/lib.rs\", srcs = glob([\"**/*.rs\"]))";

fn check(files: Vec<SourceFile>) -> Vec<Violation> {
    let repository = crate::Repository::new(&files);
    let targets = crate::targets::collect_tests(&repository);
    let mut violations = Vec::new();
    crate::reachability::inspect(&files, &repository, &targets, &mut violations);
    violations
}

fn fixture(build: &str, root: &str) -> Vec<SourceFile> {
    vec![
        source(BUILD, build),
        source(ROOT, root),
        source(CASE, TEST_BODY),
    ]
}

fn report(files: Vec<SourceFile>) -> String {
    super::support::messages(&check(files))
}

#[test]
fn glob_membership_does_not_make_an_unwired_test_module_reachable() {
    let result = report(fixture(BASE_TARGET, "pub fn value() {}"));
    assert!(result.contains(UNREACHABLE_TEST_MODULE_MESSAGE), "{result}");
    assert!(result.contains(CASE));
}

#[test]
fn explicit_path_and_nested_include_resolve_from_the_test_root() {
    let mut files = fixture(
        BASE_TARGET,
        "#[cfg(test)] #[path = \"../test_support/wiring.rs\"] mod tests;",
    );
    files.push(source(
        &format!("{PACKAGE}/test_support/wiring.rs"),
        "include!(\"../unit_tests/case.rs\");",
    ));
    assert!(check(files).is_empty());
}

#[test]
fn conventional_tests_inside_a_path_overridden_entry_are_reachable() {
    let mut files = fixture(
        BASE_TARGET,
        "#[cfg(test)] #[path = \"../unit_tests/entry.rs\"] mod tests;",
    );
    files.push(source(
        &format!("{PACKAGE}/unit_tests/entry.rs"),
        "mod case;",
    ));
    assert!(check(files).is_empty());
}

#[test]
fn conventional_modules_preserve_inline_module_search_directories() {
    let mut files = fixture(
        BASE_TARGET,
        "#[cfg(test)] mod suites { #[path = \"../../unit_tests/case.rs\"] mod case; }",
    );
    files.push(source(
        &format!("{PACKAGE}/src/unused.rs"),
        "pub struct Unused;",
    ));
    assert!(check(files).is_empty());
}

#[test]
fn a_source_outside_target_srcs_cannot_bridge_to_an_owned_test() {
    let build = "rust_test(name = \"unit\", crate_root = \"src/lib.rs\", srcs = [\"src/lib.rs\", \"unit_tests/case.rs\"])";
    let mut files = fixture(build, "#[path = \"../test_support/wiring.rs\"] mod wiring;");
    files.push(source(
        &format!("{PACKAGE}/test_support/wiring.rs"),
        "include!(\"../unit_tests/case.rs\");",
    ));
    assert!(report(files).contains(UNCERTAIN_TEST_MODULE_MESSAGE));
}

#[test]
fn wrong_test_feature_is_rejected_and_imported_feature_lists_are_resolved() {
    let root =
        "#[cfg(all(test, feature = \"unit-mode\"))] #[path = \"../unit_tests/case.rs\"] mod case;";
    assert!(report(fixture(BASE_TARGET, root)).contains(UNREACHABLE_TEST_MODULE_MESSAGE));
    let build = "load(\":features.bzl\", ENABLED = \"UNIT\")\nrust_test(name = \"unit\", crate_root = \"src/lib.rs\", crate_features = ENABLED, srcs = glob([\"**/*.rs\"]))";
    let mut files = fixture(build, root);
    files.push(source(
        &format!("{PACKAGE}/features.bzl"),
        "SUPPORT = [\"support\"]\nUNIT = SUPPORT + [\"unit-mode\"]",
    ));
    assert!(check(files).is_empty());
}

#[test]
fn separate_test_targets_do_not_union_their_feature_sets() {
    let build = "rust_test(name = \"unit\", crate_root = \"src/lib.rs\", crate_features = [\"unit-mode\"], srcs = glob([\"**/*.rs\"]))\nrust_test(name = \"integration\", crate_root = \"src/lib.rs\", crate_features = [\"integration-mode\"], srcs = glob([\"**/*.rs\"]))";
    let root = "#[cfg(all(feature = \"unit-mode\", feature = \"integration-mode\"))] #[path = \"../unit_tests/case.rs\"] mod case;";
    assert!(report(fixture(build, root)).contains(UNREACHABLE_TEST_MODULE_MESSAGE));
}

#[test]
fn test_only_library_does_not_supply_cfg_test_or_an_executable_test_root() {
    let build = "rust_library(name = \"support\", testonly = True, crate_root = \"src/lib.rs\", srcs = glob([\"**/*.rs\"]))";
    let root = "#[cfg(test)] #[path = \"../unit_tests/case.rs\"] mod case;";
    assert!(report(fixture(build, root)).contains(UNREACHABLE_TEST_MODULE_MESSAGE));
}

#[test]
fn same_line_rules_keep_their_own_test_configuration() {
    let build = "rust_library(name = \"support\", testonly = True, crate_root = \"src/lib.rs\", srcs = glob([\"**/*.rs\"])) rust_test(name = \"unit\", crate_root = \"src/lib.rs\", srcs = glob([\"**/*.rs\"]))";
    let root = "#[cfg(test)] #[path = \"../unit_tests/case.rs\"] mod case;";
    assert!(check(fixture(build, root)).is_empty());
}

#[test]
fn cfg_test_is_not_inherited_from_an_unrelated_test_crate() {
    let build = "rust_library(name = \"support\", testonly = True, crate_root = \"src/lib.rs\", srcs = glob([\"**/*.rs\"]))\nrust_test(name = \"consumer\", crate_root = \"unit_tests/consumer.rs\", srcs = [\"unit_tests/consumer.rs\"])";
    let mut files = fixture(
        build,
        "#[cfg(test)] #[path = \"../unit_tests/case.rs\"] mod case;",
    );
    files.push(source(
        &format!("{PACKAGE}/unit_tests/consumer.rs"),
        TEST_BODY,
    ));
    let result = report(files);
    assert!(result.contains(CASE));
    assert!(!result.contains("consumer.rs:"));
}

#[test]
fn inner_disabled_cfg_and_disabled_inline_module_do_not_enable_tests() {
    for root in [
        "#[cfg(any())] mod tests { include!(\"../unit_tests/case.rs\"); }",
        "#[cfg(test)] #[path = \"../unit_tests/case.rs\"] mod case;",
    ] {
        let mut files = fixture(BASE_TARGET, root);
        files.last_mut().unwrap().text = "#![cfg(not(test))]\n#[test] fn scenario() {}".into();
        assert!(!check(files).is_empty());
    }
}

#[test]
fn invalid_explicit_root_fails_closed_without_inference() {
    for root in [
        "\"src/missing.rs\"",
        "ROOT_MACRO",
        "[\"src/lib.rs\", \"unit_tests/case.rs\"]",
    ] {
        let build =
            format!("rust_test(name = \"unit\", crate_root = {root}, srcs = glob([\"**/*.rs\"]))");
        assert!(report(fixture(&build, "")).contains(UNRESOLVED_TEST_ROOT_MESSAGE));
    }
}

#[test]
fn platform_cfg_is_conservative_and_opaque_features_cannot_prove_absence() {
    let root =
        "#[cfg(all(test, target_os = \"linux\"))] #[path = \"../unit_tests/case.rs\"] mod case;";
    assert!(check(fixture(BASE_TARGET, root)).is_empty());
    let build = "rust_test(name = \"unit\", crate_root = \"src/lib.rs\", crate_features = opaque(), srcs = glob([\"**/*.rs\"]))";
    let root = "#[cfg(feature = \"unknown-mode\")] #[path = \"../unit_tests/case.rs\"] mod case;";
    assert!(check(fixture(build, root)).is_empty());
}

#[test]
fn roots_without_test_sources_and_targets_without_explicit_roots_are_out_of_scope() {
    let build = "rust_test(name = \"unit\", srcs = glob([\"**/*.rs\"]))";
    assert!(check(fixture(build, "")).is_empty());
    let build = "rust_test(name = \"unit\", crate_root = \"src/lib.rs\", srcs = [\"src/lib.rs\"])";
    assert!(check(fixture(build, "")).is_empty());
}
