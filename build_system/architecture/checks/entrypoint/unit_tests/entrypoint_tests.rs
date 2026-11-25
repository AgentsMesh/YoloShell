use super::{
    entrypoint_fixture::*,
    support::{inspect, messages, source},
};
use crate::constants as contract;

#[test]
fn sole_product_entry_and_literal_test_only_variants_are_accepted() {
    let report = report(format!("{ENTRY}{TEST_BINARY}"));
    assert!(report.is_empty(), "{report}");
}

#[test]
fn an_extra_core_executable_cannot_hide_in_a_different_component() {
    let mut inventory = files(ENTRY);
    inventory.push(source(OTHER_BUILD, binary("helper", "")));
    inventory.push(source(OTHER_MAIN, MAIN_SOURCE));
    let report = messages(&inspect(inventory));
    assert!(report.contains(contract::EXECUTABLE_POLICY), "{report}");
    assert!(report.contains(OTHER_BUILD), "{report}");
}

#[test]
fn removing_or_obscuring_testonly_turns_a_fixture_into_a_forbidden_product() {
    for value in [
        "",
        "testonly = False,",
        "testonly = \"True\",",
        "testonly = FIXTURE,",
    ] {
        let report = report(format!("{ENTRY}{}", binary("fixture", value)));
        assert!(report.contains(contract::EXECUTABLE_POLICY), "{report}");
    }
}

#[test]
fn canonical_entry_cannot_disappear_behind_a_computed_testonly_value() {
    for value in ["FIXTURE", "select({})", "\"False\""] {
        let build = ENTRY.replace(
            "name = \"yolo\",",
            &format!("name = \"yolo\", testonly = {value},"),
        );
        let report = report(format!("FIXTURE = True\n{build}"));
        assert!(report.contains(contract::EXECUTABLE_POLICY), "{report}");
    }
    let report = report(ENTRY.replace("name = \"yolo\",", "name = \"yolo\", testonly = False,"));
    assert!(report.is_empty(), "{report}");
}

#[test]
fn missing_duplicate_and_renamed_product_entries_are_rejected() {
    for changed in [
        String::new(),
        format!("{ENTRY}{ENTRY}"),
        ENTRY.replace("\"yolo\"", "\"helper\""),
    ] {
        let report = report(changed);
        assert!(report.contains(contract::EXECUTABLE_POLICY), "{report}");
    }
}

#[test]
fn rules_rust_aliases_cannot_bypass_the_executable_census() {
    let aliased = format!(
        "load(\"@rules_rust//rust:defs.bzl\", entry = \"rust_binary\")\n{}",
        ENTRY.replace("rust_binary(", "entry(")
    );
    let valid = report(&aliased);
    assert!(valid.is_empty(), "{valid}");
    let invalid = report(format!(
        "{aliased}{}",
        binary("helper", "").replace("rust_binary(", "entry(")
    ));
    assert!(invalid.contains(contract::EXECUTABLE_POLICY), "{invalid}");
}

#[test]
fn untrusted_or_ambiguous_binary_rule_origins_fail_closed() {
    for load in [
        "load(\"//custom:rules.bzl\", entry = \"rust_binary\")\n",
        "load(\"@rules_rust//rust:defs.bzl\", entry = \"rust_binary\", entry = \"rust_library\")\n",
    ] {
        let report = report(format!("{load}{ENTRY}"));
        assert!(
            report.contains(contract::RULE_RESOLUTION_POLICY),
            "{report}"
        );
    }
}

#[test]
fn composition_root_cannot_absorb_more_source_files_or_lines() {
    let mut inventory = files(ENTRY);
    for index in 0..contract::MAX_SOURCES {
        inventory.push(source(
            format!("{}part_{index}.rs", contract::SOURCE_PREFIX).as_str(),
            "// source\n",
        ));
    }
    let report = messages(&inspect(inventory));
    assert!(report.contains(contract::COMPOSITION_POLICY), "{report}");
    let mut inventory = files(ENTRY);
    inventory
        .iter_mut()
        .find(|file| file.relative == MAIN)
        .unwrap()
        .text = format!(
        "{MAIN_SOURCE}{}",
        "// bounded entry source\n".repeat(contract::MAX_LINES)
    );
    let report = messages(&inspect(inventory));
    assert!(report.contains(contract::COMPOSITION_POLICY), "{report}");
}

#[test]
fn entry_root_and_source_expression_must_remain_explicit_and_local() {
    for build in [
        ENTRY.replace("crate_root = \"src/main.rs\",", ""),
        ENTRY.replace("glob([\"src/*.rs\"])", "IMPORTED_SOURCES"),
        ENTRY.replace(
            "glob([\"src/*.rs\"])",
            "[\"//core/crates/other:src/main.rs\"]",
        ),
    ] {
        let report = report(build);
        assert!(report.contains(contract::COMPOSITION_POLICY), "{report}");
    }
}
