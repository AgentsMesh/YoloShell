use super::constants::{
    EXAMPLE_PACKAGE as PACKAGE, ISOLATED_RUST_TEST_RULE as ISOLATED_RUST_TEST,
    RUST_TEST_RULE as RUST_TEST, TEST_BUDGET_VIOLATION as VIOLATION,
    UNIT_TEST_TARGET as UNIT_TARGET, mixed_target_build,
};
use super::support::{inspect, messages, source};
use crate::MAX_BUILD_SYSTEM_TEST_SOURCES_PER_TARGET;

#[test]
fn rust_test_variants_share_the_test_owned_source_budget() {
    for rule in [RUST_TEST, ISOLATED_RUST_TEST] {
        let report = fixture(
            PACKAGE,
            rule,
            UNIT_TARGET,
            MAX_BUILD_SYSTEM_TEST_SOURCES_PER_TARGET + 1,
        );
        assert!(report.contains(VIOLATION), "{rule}: {report}");
    }
}

#[test]
fn ordinary_test_target_at_the_budget_is_accepted() {
    let report = fixture(
        PACKAGE,
        RUST_TEST,
        UNIT_TARGET,
        MAX_BUILD_SYSTEM_TEST_SOURCES_PER_TARGET,
    );
    assert!(!report.contains(VIOLATION), "{report}");
}

#[test]
fn test_only_library_uses_the_test_budget() {
    let source_count = MAX_BUILD_SYSTEM_TEST_SOURCES_PER_TARGET + 1;
    let names = source_names(source_count);
    let build = format!("rust_library(name = \"test_support\", testonly = True, srcs = [{names}])");
    let report = inspect_fixture(PACKAGE, build, source_count);
    assert!(report.contains(VIOLATION), "{report}");
}

#[test]
fn unresolved_test_sources_fail_closed() {
    let build = concat!(
        "rust_test(name = \"unit_tests\", ",
        "srcs = select({\"//conditions:default\": glob([\"unit_tests/*.rs\"])}))",
    );
    let report = messages(&inspect(vec![
        source(&format!("{PACKAGE}/BUILD.bazel"), build),
        source(
            &format!("{PACKAGE}/unit_tests/case_tests.rs"),
            "#[test] fn case() {}",
        ),
    ]));
    assert!(report.contains("must be statically resolvable"), "{report}");
}

#[test]
fn orphaned_test_sources_fail_closed() {
    let report = messages(&inspect(vec![
        source(
            &format!("{PACKAGE}/BUILD.bazel"),
            "rust_test(name = \"unit_tests\", srcs = [\"unit_tests/owned.rs\"])",
        ),
        source(
            &format!("{PACKAGE}/unit_tests/owned.rs"),
            "#[test] fn owned() {}",
        ),
        source(
            &format!("{PACKAGE}/unit_tests/orphaned.rs"),
            "#[test] fn orphaned() {}",
        ),
    ]));
    assert!(
        report.contains("must belong to at least one local test target"),
        "{report}"
    );
}

#[test]
fn production_sources_do_not_consume_the_test_owned_budget() {
    let production_count = crate::MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_TARGET;
    let test_count = MAX_BUILD_SYSTEM_TEST_SOURCES_PER_TARGET;
    let production = (0..production_count)
        .map(|index| format!("\"src/source_{index}.rs\""))
        .collect::<Vec<_>>()
        .join(",");
    let tests = source_names(test_count);
    let build = mixed_target_build(&production, &tests);
    let mut files = vec![source(&format!("{PACKAGE}/BUILD.bazel"), build)];
    files.extend((0..production_count).map(|index| {
        source(
            &format!("{PACKAGE}/src/source_{index}.rs"),
            "pub struct Value;",
        )
    }));
    files.extend((0..test_count).map(|index| {
        source(
            &format!("{PACKAGE}/unit_tests/case_{index}.rs"),
            "#[test] fn case() {}",
        )
    }));
    let report = messages(&inspect(files));
    assert!(!report.contains(VIOLATION), "{report}");
}

fn fixture(package: &str, rule: &str, target: &str, source_count: usize) -> String {
    let names = source_names(source_count);
    let build = format!("{rule}(name = \"{target}\", srcs = [{names}])");
    inspect_fixture(package, build, source_count)
}

fn source_names(source_count: usize) -> String {
    (0..source_count)
        .map(|index| format!("\"unit_tests/case_{index}.rs\""))
        .collect::<Vec<_>>()
        .join(",")
}

fn inspect_fixture(package: &str, build: String, source_count: usize) -> String {
    let mut files = vec![source(&format!("{package}/BUILD.bazel"), build)];
    files.extend((0..source_count).map(|index| {
        source(
            &format!("{package}/unit_tests/case_{index}.rs"),
            "#[test] fn case() {}",
        )
    }));
    messages(&inspect(files))
}
