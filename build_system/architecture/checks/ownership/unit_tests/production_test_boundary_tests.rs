use super::support::{inspect, messages, source};
use crate::constants::{
    INTEGRATION_TEST_DIRECTORY, PRODUCTION_TEST_SOURCE_MESSAGE, RUST_BINARY_RULE,
    RUST_LIBRARY_RULE, TEST_SUPPORT_DIRECTORY, UNIT_TEST_DIRECTORY,
};

const PACKAGE: &str = "build_system/example";
const SOURCE_FILE: &str = "entry.rs";
const SOURCE_TEXT: &str = "pub fn fixture() {}";
const PRODUCTION_RULES: &[&str] = &[RUST_LIBRARY_RULE, RUST_BINARY_RULE];

#[test]
fn test_only_ownership_does_not_allow_production_to_compile_the_same_source() {
    for rule in PRODUCTION_RULES {
        for directory in [
            UNIT_TEST_DIRECTORY,
            INTEGRATION_TEST_DIRECTORY,
            TEST_SUPPORT_DIRECTORY,
        ] {
            let relative = format!("{directory}/{SOURCE_FILE}");
            let build = format!(
                "{rule}(name = \"production\", srcs = [\"{relative}\"])\n\
                 rust_library(name = \"fixture\", testonly = True, srcs = [\"{relative}\"])"
            );
            let report = fixture(&relative, &build);
            assert!(report.contains(PRODUCTION_TEST_SOURCE_MESSAGE), "{report}");
        }
    }
}

#[test]
fn filegroups_cannot_hide_test_sources_from_the_production_boundary() {
    let relative = format!("{TEST_SUPPORT_DIRECTORY}/{SOURCE_FILE}");
    let build = format!(
        "filegroup(name = \"fixture_sources\", srcs = [\"{relative}\"])\n\
         rust_library(name = \"production\", srcs = [\":fixture_sources\"])\n\
         rust_test(name = \"unit_tests\", srcs = [\"{relative}\"])"
    );
    let report = fixture(&relative, &build);
    assert!(report.contains(PRODUCTION_TEST_SOURCE_MESSAGE), "{report}");
}

#[test]
fn test_only_libraries_can_compile_test_support() {
    let relative = format!("{TEST_SUPPORT_DIRECTORY}/{SOURCE_FILE}");
    let build =
        format!("rust_library(name = \"fixture\", testonly = True, srcs = [\"{relative}\"])");
    let report = fixture(&relative, &build);
    assert!(report.is_empty(), "{report}");
}

#[test]
fn production_sources_can_be_recompiled_for_unit_tests() {
    let relative = format!("src/{SOURCE_FILE}");
    let build = format!(
        "rust_library(name = \"production\", srcs = [\"{relative}\"])\n\
         rust_test(name = \"unit_tests\", srcs = [\"{relative}\"])"
    );
    let report = fixture(&relative, &build);
    assert!(report.is_empty(), "{report}");
}

#[test]
fn fixture_only_libraries_and_binaries_own_their_ordinary_source_files() {
    for rule in PRODUCTION_RULES {
        let relative = format!("src/{SOURCE_FILE}");
        let build =
            format!("{rule}(name = \"fixture\", testonly = True, srcs = [\"{relative}\"])",);
        let report = fixture(&relative, &build);
        assert!(report.is_empty(), "{report}");
    }
}

#[test]
fn compiling_orphan_product_sources_as_a_unit_test_does_not_hide_the_missing_owner() {
    let relative = format!("src/{SOURCE_FILE}");
    let report = fixture(
        &relative,
        &format!("rust_test(name = \"unit_tests\", srcs = [\"{relative}\"])"),
    );
    assert!(
        report.contains(crate::constants::MISSING_OWNER_MESSAGE),
        "{report}"
    );
}

#[test]
fn helper_recompilation_does_not_remove_shared_product_sources_from_ownership_checks() {
    let relative = format!("src/{SOURCE_FILE}");
    let build = format!(
        "rust_library(name = \"production\", srcs = [\"{relative}\"])\n\
         rust_library(name = \"duplicate\", srcs = [\"{relative}\"])\n\
         rust_library(name = \"fixture\", testonly = True, srcs = [\"{relative}\"])"
    );
    let report = fixture(&relative, &build);
    assert!(
        report.contains(crate::constants::DUPLICATE_OWNER_MESSAGE),
        "{report}"
    );
}

fn fixture(relative: &str, build: &str) -> String {
    messages(&inspect(vec![
        source(&format!("{PACKAGE}/BUILD.bazel"), build),
        source(&format!("{PACKAGE}/{relative}"), SOURCE_TEXT),
    ]))
}
