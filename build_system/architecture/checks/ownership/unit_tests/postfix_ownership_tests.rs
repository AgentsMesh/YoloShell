use super::support::{inspect, messages, source};

const BUILD: &str = "build_system/component/BUILD.bazel";
const TEST_SOURCE: &str = "build_system/component/integration_tests/lib.rs";
const TEST_BODY: &str = "#[test] fn exercise() {}";
const SOURCE_ATTRIBUTE: &str = "srcs = [\"integration_tests/lib.rs\"]";
const COVERAGE_ATTRIBUTE: &str = "coverage_objects = depset([\"//core:yolo\"]).to_list()";

#[test]
fn postfix_in_an_unrelated_attribute_preserves_the_entire_isolated_test_target() {
    for attributes in [
        format!("{COVERAGE_ATTRIBUTE}, {SOURCE_ATTRIBUTE}"),
        format!("{SOURCE_ATTRIBUTE}, {COVERAGE_ATTRIBUTE}"),
        format!("{SOURCE_ATTRIBUTE}, data = [] + depset([\"//core:yolo\"]).to_list()"),
    ] {
        let build = format!(
            "isolated_rust_test(name = \"integration_tests\", crate_root = \"integration_tests/lib.rs\", {attributes})"
        );
        let violations = inspect(vec![source(BUILD, build), source(TEST_SOURCE, TEST_BODY)]);
        assert!(violations.is_empty(), "{}", messages(&violations));
    }
}

#[test]
fn unsupported_postfix_sources_still_fail_static_source_resolution() {
    for expression in [
        "depset([\"integration_tests/lib.rs\"]).to_list()",
        "factory().sources.value()",
    ] {
        let build = format!(
            "isolated_rust_test(name = \"integration_tests\", crate_root = \"integration_tests/lib.rs\", srcs = {expression})"
        );
        let report = messages(&inspect(vec![
            source(BUILD, build),
            source(TEST_SOURCE, TEST_BODY),
        ]));
        assert!(report.contains("must be statically resolvable"), "{report}");
        assert!(report.contains("must belong to at least one"), "{report}");
    }
}
