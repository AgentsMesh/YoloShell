use crate::coverage_data::summaries;
use crate::coverage_json::Summary;
use crate::instance_fixture::{
    FILE_INDEX, ROOT, SINGLE_METRIC, SOURCE, TEST_SOURCE, data, function,
};
use std::path::Path;

#[test]
fn summaries_cannot_duplicate_a_production_source_under_a_normalized_alias() {
    let mut observed = data(vec![function()]);
    let mut alias = data(vec![function()]).files.remove(FILE_INDEX as usize);
    alias.filename = format!("{ROOT}/{SOURCE}");
    observed.files.push(alias);
    assert_eq!(
        summaries(&observed, Path::new(ROOT)).unwrap_err(),
        format!("duplicate llvm-cov summary for {SOURCE}")
    );
}

#[test]
fn ignored_test_summaries_cannot_substitute_for_missing_production_evidence() {
    let mut observed = data(vec![function()]);
    observed.files[FILE_INDEX as usize].filename = TEST_SOURCE.into();
    assert_eq!(
        summaries(&observed, Path::new(ROOT)).unwrap_err(),
        "llvm-cov JSON contains no production sources"
    );
}

#[test]
fn every_summary_rejects_a_covered_count_above_its_denominator() {
    let mutations: &[(&str, fn(&mut Summary))] = &[
        ("lines", |summary| {
            summary.lines.covered = summary.lines.count + SINGLE_METRIC
        }),
        ("functions", |summary| {
            summary.functions.covered = summary.functions.count + SINGLE_METRIC
        }),
        ("regions", |summary| {
            summary.regions.covered = summary.regions.count + SINGLE_METRIC
        }),
        ("branches", |summary| {
            summary.branches.covered = summary.branches.count + SINGLE_METRIC
        }),
    ];
    for (label, mutate) in mutations {
        let mut observed = data(vec![function()]);
        mutate(&mut observed.files[FILE_INDEX as usize].summary);
        let error = summaries(&observed, Path::new(ROOT)).unwrap_err();
        assert!(error.contains(label), "{error}");
        assert!(error.contains(SOURCE), "{error}");
    }
}

#[test]
fn export_requires_one_complete_data_object_before_reading_its_evidence() {
    const EMPTY_DATA: &str = r#"{"files":[],"functions":[]}"#;
    for entries in [String::new(), format!("{EMPTY_DATA},{EMPTY_DATA}")] {
        let report = format!(r#"{{"data":[{entries}]}}"#);
        let error = match crate::coverage_json::parse(&report) {
            Ok(_) => panic!("ambiguous or absent coverage data was accepted"),
            Err(error) => error,
        };
        assert!(error.contains("data entries instead of one"), "{error}");
    }
    assert!(crate::coverage_json::parse(&format!(r#"{{"data":[{EMPTY_DATA}]}}"#)).is_ok());
}
