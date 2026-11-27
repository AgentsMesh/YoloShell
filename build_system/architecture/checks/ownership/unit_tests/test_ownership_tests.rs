use super::constants::{INTEGRATION_TEST_TARGET, UNIT_TEST_TARGET};
use super::support::{inspect, messages, source};
use crate::constants::{INTEGRATION_TEST_DIRECTORY, TEST_SUPPORT_DIRECTORY, UNIT_TEST_DIRECTORY};

const PACKAGE: &str = "core/crates/model";
const TEST_FILE: &str = "model_tests.rs";
const FIXTURE_PACKAGE: &str = "integration_tests/runtime";

#[test]
fn fixture_unit_tests_are_classified_relative_to_their_package() {
    let build = format!(
        "rust_test(name = \"{UNIT_TEST_TARGET}\", srcs = [\"fixture.rs\", \"{UNIT_TEST_DIRECTORY}/{TEST_FILE}\"])"
    );
    let report = messages(&inspect(vec![
        source(&format!("{FIXTURE_PACKAGE}/BUILD.bazel"), build),
        source(
            &format!("{FIXTURE_PACKAGE}/fixture.rs"),
            "pub fn fixture() {}",
        ),
        source(
            &format!("{FIXTURE_PACKAGE}/{UNIT_TEST_DIRECTORY}/{TEST_FILE}"),
            "#[test] fn unit() {}",
        ),
    ]));
    assert!(
        !report.contains("require separate Rust test targets"),
        "{report}"
    );
}

fn package_file(relative: &str) -> String {
    format!("{PACKAGE}/{relative}")
}

fn test_file(directory: &str) -> String {
    package_file(&format!("{directory}/{TEST_FILE}"))
}

#[test]
fn component_test_sources_require_a_local_owner() {
    let report = messages(&inspect(vec![
        source(&package_file("BUILD.bazel"), ""),
        source(&test_file(UNIT_TEST_DIRECTORY), "#[test] fn model() {}"),
    ]));
    assert!(
        report.contains("must belong to at least one local test target"),
        "{report}"
    );
}

#[test]
fn test_targets_cannot_aggregate_another_package() {
    let source_target = format!("{UNIT_TEST_TARGET}_sources");
    let facade =
        format!("rust_test(name = \"all_tests\", srcs = [\"//{PACKAGE}:{source_target}\"])");
    let component = format!(
        "filegroup(name = \"{source_target}\", testonly = True, \
         srcs = [\"{UNIT_TEST_DIRECTORY}/{TEST_FILE}\"])"
    );
    let report = messages(&inspect(vec![
        source("core/BUILD.bazel", facade),
        source(&package_file("BUILD.bazel"), component),
        source(&test_file(UNIT_TEST_DIRECTORY), "#[test] fn model() {}"),
    ]));
    assert!(report.contains("another Bazel package"), "{report}");
}

#[test]
fn unit_and_integration_sources_require_distinct_targets() {
    let build = format!(
        "rust_test(name = \"mixed_tests\", srcs = [\
         \"{UNIT_TEST_DIRECTORY}/{TEST_FILE}\", \
         \"{INTEGRATION_TEST_DIRECTORY}/{TEST_FILE}\"])"
    );
    let report = messages(&inspect(vec![
        source(&package_file("BUILD.bazel"), build),
        source(&test_file(UNIT_TEST_DIRECTORY), "#[test] fn unit() {}"),
        source(
            &test_file(INTEGRATION_TEST_DIRECTORY),
            "#[test] fn integration() {}",
        ),
    ]));
    assert!(
        report.contains("require separate Rust test targets"),
        "{report}"
    );
}

#[test]
fn local_separated_targets_and_shared_support_are_accepted() {
    let support = format!("{TEST_SUPPORT_DIRECTORY}/model.rs");
    let build = format!(
        "rust_test(name = \"{UNIT_TEST_TARGET}\", srcs = [\
         \"{UNIT_TEST_DIRECTORY}/{TEST_FILE}\", \"{support}\"])\n\
         rust_test(name = \"{INTEGRATION_TEST_TARGET}\", srcs = [\
         \"{INTEGRATION_TEST_DIRECTORY}/{TEST_FILE}\", \"{support}\"])"
    );
    let report = messages(&inspect(vec![
        source(&package_file("BUILD.bazel"), build),
        source(&test_file(UNIT_TEST_DIRECTORY), "#[test] fn unit() {}"),
        source(
            &test_file(INTEGRATION_TEST_DIRECTORY),
            "#[test] fn integration() {}",
        ),
        source(&package_file(&support), "pub fn fixture() {}"),
    ]));
    assert!(report.is_empty(), "{report}");
}
