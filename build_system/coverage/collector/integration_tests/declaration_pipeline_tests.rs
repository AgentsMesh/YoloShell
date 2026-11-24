use crate::constants::{COVERAGE_MANIFEST_ENV, DECLARATION_ONLY_DETAIL, TEST_ONLY_DETAIL};
use crate::declaration_pipeline_fixture::*;
use crate::entrypoint_fixture::{FIXTURE_FAILURE_ENV, REPORT_STAGE};
use crate::instance_fixture::TEST_SOURCE;
use crate::test_constants::{FIXTURE_SOURCE, LCOV_FILE, TEST_SUPPORT_SOURCE};

#[test]
fn verified_declaration_only_pipeline_finishes_without_function_report_substitution() {
    let mut fixture = prepare();
    fixture.command.env(FIXTURE_FAILURE_ENV, REPORT_STAGE);
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains(DECLARATION_ONLY_DETAIL));
    assert!(
        std::fs::read_to_string(fixture.report())
            .unwrap()
            .contains(TEST_SOURCE)
    );
    assert!(!fixture.marker().exists());
    assert!(!fixture.profile().exists());
}

#[test]
fn declaration_only_collection_requires_a_readable_utf8_source_manifest() {
    let mut absent = prepare();
    absent.command.env_remove(COVERAGE_MANIFEST_ENV);
    absent.fails("declaration-only coverage requires Bazel's source manifest");

    let mut missing = prepare();
    std::fs::remove_file(missing.root().join(MANIFEST_FILE)).unwrap();
    missing.fails("cannot read declaration evidence");

    let mut invalid = prepare();
    std::fs::write(invalid.root().join(MANIFEST_FILE), INVALID_UTF8).unwrap();
    invalid.fails("invalid coverage source manifest");
}

#[test]
fn declaration_only_collection_revalidates_the_runfile_source_fingerprints() {
    let mut missing = prepare();
    std::fs::remove_file(first_source(&missing)).unwrap();
    missing.fails("cannot read declaration evidence");

    let mut changed = prepare();
    std::fs::write(first_source(&changed), UNDECLARED_SOURCE_CONTENT).unwrap();
    changed.fails("declaration-only source changed and requires review");
}

#[test]
fn test_only_pipeline_preserves_real_test_records_without_production_metric_tokens() {
    let mut fixture = prepare();
    std::fs::write(
        fixture.root().join(MANIFEST_FILE),
        format!("{TEST_SOURCE}\n{TEST_SUPPORT_SOURCE}"),
    )
    .unwrap();
    fixture.command.env(FIXTURE_FAILURE_ENV, REPORT_STAGE);
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains(TEST_ONLY_DETAIL));
    let report = std::fs::read_to_string(fixture.report()).unwrap();
    assert!(report.contains(TEST_SOURCE));
    assert!(!report.contains(yolo_coverage_protocol::TOKEN_ROOT));
    assert!(!fixture.marker().exists());
    assert!(!fixture.profile().exists());
}

#[test]
fn test_only_manifest_cannot_hide_missing_production_or_lcov_production_records() {
    let mut manifest = prepare();
    std::fs::write(
        manifest.root().join(MANIFEST_FILE),
        format!("{TEST_SOURCE}\n{FIXTURE_SOURCE}"),
    )
    .unwrap();
    manifest.fails("missing production mappings have no declaration-only source contract");

    let mut lcov = prepare();
    std::fs::write(lcov.root().join(MANIFEST_FILE), TEST_SOURCE).unwrap();
    let path = lcov.root().join(LCOV_FILE);
    let report = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, report.replace(TEST_SOURCE, FIXTURE_SOURCE)).unwrap();
    lcov.fails("LCOV production sources differ from JSON summaries");
}
