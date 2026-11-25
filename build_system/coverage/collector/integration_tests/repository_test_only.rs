use crate::constants::TEST_ONLY_DETAIL;
use crate::declaration_pipeline_fixture::{MANIFEST_FILE, prepare_test_only};
use crate::test_constants::*;

const FILTERED_DEPENDENCY_SOURCE: &str =
    "external/rules_rust++crate+crates__serde_json-1.0.151/src/error.rs";

#[test]
fn repository_test_only_collection_retains_test_records_without_production_metrics() {
    let mut fixture = prepare_test_only(REPOSITORY_TEST_SOURCE, REPOSITORY_SUPPORT_SOURCE);
    // There is no production function inventory to substitute for LLVM evidence.
    fixture.command.env(FIXTURE_FAILURE_ENV, REPORT_STAGE);
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains(TEST_ONLY_DETAIL));
    let report = std::fs::read_to_string(fixture.report()).unwrap();
    assert!(report.contains(REPOSITORY_TEST_SOURCE));
    assert!(!report.contains(yolo_coverage_protocol::TOKEN_ROOT));
    assert!(!fixture.marker().exists());
    assert!(!fixture.profile().exists());
}

#[test]
fn absent_repository_sources_or_production_members_cannot_complete_collection() {
    for manifest in [
        String::new(),
        REPOSITORY_SUPPORT_SOURCE.to_owned(),
        format!("{REPOSITORY_TEST_SOURCE}\n{FIXTURE_SOURCE}"),
    ] {
        let mut fixture = prepare_test_only(REPOSITORY_TEST_SOURCE, REPOSITORY_SUPPORT_SOURCE);
        std::fs::write(fixture.root().join(MANIFEST_FILE), manifest).unwrap();
        fixture.fails("missing production mappings have no declaration-only source contract");
    }
}

#[test]
fn repository_test_only_collection_does_not_bypass_llvm_or_missing_raw_profile_failures() {
    for (stage, error) in [
        (MERGE_STAGE, "llvm-profdata failed"),
        (LCOV_STAGE, "llvm-cov failed"),
        (JSON_STAGE, "llvm-cov failed"),
    ] {
        let mut fixture = prepare_test_only(REPOSITORY_TEST_SOURCE, REPOSITORY_SUPPORT_SOURCE);
        fixture.command.env(FIXTURE_FAILURE_ENV, stage);
        fixture.fails(error);
    }
    let mut fixture = prepare_test_only(REPOSITORY_TEST_SOURCE, REPOSITORY_SUPPORT_SOURCE);
    std::fs::remove_file(fixture.root().join(FIXTURE_COVERAGE).join(FIXTURE_RAW)).unwrap();
    fixture.fails("coverage run produced no raw profiles");
}

#[test]
fn repository_test_manifest_cannot_hide_undeclared_llvm_or_production_lcov_mappings() {
    let mut undeclared = prepare_test_only(REPOSITORY_TEST_SOURCE, REPOSITORY_SUPPORT_SOURCE);
    let path = undeclared.root().join(JSON_FILE);
    let json = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        path,
        json.replace(REPOSITORY_TEST_SOURCE, OTHER_REPOSITORY_TEST_SOURCE),
    )
    .unwrap();
    undeclared.fails("test-only coverage contains an undeclared source mapping");

    let mut production = prepare_test_only(REPOSITORY_TEST_SOURCE, REPOSITORY_SUPPORT_SOURCE);
    let path = production.root().join(LCOV_FILE);
    let lcov = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, lcov.replace(REPOSITORY_TEST_SOURCE, FIXTURE_SOURCE)).unwrap();
    production.fails("LCOV production sources differ from JSON summaries");
}

#[test]
fn llvm_filtered_dependency_functions_preserve_complete_test_only_collection() {
    let mut fixture = prepare_test_only(REPOSITORY_TEST_SOURCE, REPOSITORY_SUPPORT_SOURCE);
    let path = fixture.root().join(JSON_FILE);
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let functions = json["data"][0]["functions"].as_array_mut().unwrap();
    let mut dependency = functions[0].clone();
    dependency["filenames"] = serde_json::json!([FILTERED_DEPENDENCY_SOURCE]);
    functions.push(dependency);
    std::fs::write(path, serde_json::to_vec(&json).unwrap()).unwrap();
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = std::fs::read_to_string(fixture.report()).unwrap();
    assert!(report.contains(REPOSITORY_TEST_SOURCE));
    assert!(!report.contains(yolo_coverage_protocol::TOKEN_ROOT));
    assert!(!fixture.marker().exists());
}
