use super::{
    entrypoint_fixture::*,
    support::{inspect, messages, source},
};
use crate::constants as contract;

const MACRO_SOURCE: &str = "build_system/bazel/rust/runtime_test_suite.bzl";
const MACRO_LOAD: &str =
    "load(\"//build_system/bazel/rust:runtime_test_suite.bzl\", \"runtime_test_suite\")\n";
const MACRO_CALL: &str = "runtime_test_suite([])\n";
const SAFE_MACRO: &str =
    "def runtime_test_suite(tests):\n    native.test_suite(name=\"tests\", tests=tests)\n";

#[test]
fn assignment_aliases_and_indirect_rule_calls_cannot_hide_extra_binaries() {
    for extra in [
        "extra = rust_binary\nextra(name=\"second\", crate_root=\"src/main.rs\", srcs=[\"src/main.rs\"])\n",
        "alias = rust_binary\nalias(name=\"second\", crate_root=\"src/main.rs\", srcs=[\"src/main.rs\"])\n",
        "(rust_binary)(name=\"second\", srcs=[\"src/main.rs\"])\n",
        "[rust_binary][0](name=\"second\", srcs=[\"src/main.rs\"])\n",
        "filegroup(name=\"hidden\", srcs=extra())\n",
    ] {
        let report = report(format!("{ENTRY}{extra}"));
        assert!(
            report.contains(contract::CALLABLE_POLICY)
                || report.contains(contract::RULE_RESOLUTION_POLICY),
            "{report}"
        );
    }
}

#[test]
fn unreviewed_local_macro_cannot_emit_an_unseen_production_binary() {
    let mut inventory = files(format!(
        "load(\"//build_system:extra.bzl\", \"extra_binary\")\n{ENTRY}extra_binary(name=\"second\")\n"
    ));
    inventory.push(source(
        "build_system/extra.bzl",
        super::constants::UNREVIEWED_BINARY_MACRO,
    ));
    let report = messages(&inspect(inventory));
    assert!(report.contains(contract::CALLABLE_POLICY), "{report}");
}

#[test]
fn reviewed_macro_name_from_a_different_source_is_rejected() {
    let mut inventory = files(format!(
        "load(\"//build_system:extra.bzl\", \"runtime_test_suite\")\n{ENTRY}{MACRO_CALL}"
    ));
    inventory.push(source("build_system/extra.bzl", SAFE_MACRO));
    let report = messages(&inspect(inventory));
    assert!(report.contains(contract::CALLABLE_POLICY), "{report}");
}

#[test]
fn reviewed_test_macros_and_pure_value_functions_keep_their_exact_origins() {
    let mut inventory = files(format!("{MACRO_LOAD}{ENTRY}{MACRO_CALL}"));
    inventory.push(source(MACRO_SOURCE, SAFE_MACRO));
    let report = messages(&inspect(inventory));
    assert!(report.is_empty(), "{report}");
    let mut inventory = files(format!(
        "load(\"//build_system/bazel/rust:component_test_graph.bzl\", \"component_test_dependencies\")\nVALUE = component_test_dependencies([])\n{ENTRY}"
    ));
    inventory.push(source(
        "build_system/bazel/rust/component_test_graph.bzl",
        "def component_test_dependencies(deps):\n    return deps\n",
    ));
    let report = messages(&inspect(inventory));
    assert!(report.is_empty(), "{report}");
}

#[test]
fn adding_a_product_binary_to_a_reviewed_macro_is_rejected() {
    for attribute in ["", "testonly = False,"] {
        let mut inventory = files(format!("{MACRO_LOAD}{ENTRY}{MACRO_CALL}"));
        inventory.push(source(
            MACRO_SOURCE,
            format!("{SAFE_MACRO}{}", binary("second", attribute)),
        ));
        let report = messages(&inspect(inventory));
        assert!(report.contains(contract::EXECUTABLE_POLICY), "{report}");
    }
    let mut inventory = files(format!("{MACRO_LOAD}{ENTRY}{MACRO_CALL}"));
    inventory.push(source(MACRO_SOURCE, format!("{SAFE_MACRO}{TEST_BINARY}")));
    let report = messages(&inspect(inventory));
    assert!(report.is_empty(), "{report}");
}

#[test]
fn reviewed_macro_cannot_delegate_to_an_unreviewed_import_or_callable_alias() {
    for mutation in [
        "load(\"//build_system:extra.bzl\", \"extra_binary\")\nextra_binary(name=\"second\")\n",
        "extra = rust_binary\nextra(name=\"second\", srcs=[\"src/main.rs\"])\n",
        "[rust_binary][0](name=\"second\", srcs=[\"src/main.rs\"])\n",
        "load(\"//custom:rules.bzl\", hidden = \"rust_binary\")\nhidden(name=\"second\")\n",
    ] {
        let mut inventory = files(format!("{MACRO_LOAD}{ENTRY}{MACRO_CALL}"));
        inventory.push(source(MACRO_SOURCE, format!("{SAFE_MACRO}{mutation}")));
        inventory.push(source(
            "build_system/extra.bzl",
            "def extra_binary(name):\n    rust_binary(name=name, srcs=[\"src/main.rs\"])\n",
        ));
        let report = messages(&inspect(inventory));
        assert!(report.contains(contract::EXECUTABLE_POLICY), "{report}");
    }
}

#[test]
fn only_direct_builtin_depset_conversion_is_a_safe_build_value_call() {
    let accepted = report(format!(
        "VALUES = depset([\"//core:yolo\"]).to_list()\n{ENTRY}"
    ));
    assert!(accepted.is_empty(), "{accepted}");
    for statement in [
        "VALUES = factory().to_list()",
        "VALUES = values.to_list()",
        "VALUES = depset([]).emit()",
        "depset = rust_binary\nVALUES = depset([]).to_list()",
    ] {
        let report = report(format!("{statement}\n{ENTRY}"));
        assert!(report.contains(contract::CALLABLE_POLICY), "{report}");
    }
}

#[test]
fn parenthesized_control_flow_is_not_a_callable_but_unknown_nested_calls_still_fail() {
    let conditional = "def runtime_test_suite(tests):\n    if (not (tests) or (tests)):\n        native.test_suite(name=\"tests\", tests=tests)\n    elif (tests and (tests)):\n        return (tests)\n";
    let mut inventory = files(format!("{MACRO_LOAD}{ENTRY}{MACRO_CALL}"));
    inventory.push(source(MACRO_SOURCE, conditional));
    let report = messages(&inspect(inventory));
    assert!(report.is_empty(), "{report}");
    let mut inventory = files(format!("{MACRO_LOAD}{ENTRY}{MACRO_CALL}"));
    inventory.push(source(
        MACRO_SOURCE,
        conditional.replace("if (not (tests)", "if (unknown(tests)"),
    ));
    let report = messages(&inspect(inventory));
    assert!(report.contains(contract::EXECUTABLE_POLICY), "{report}");
}
