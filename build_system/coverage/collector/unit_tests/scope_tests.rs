use crate::coverage_json::Data;
use crate::declaration_fixture::{data_for, read};
use crate::instance_fixture::{BAZEL_WITNESS_PREFIX, FILE_INDEX, ROOT, SOURCE};
use crate::test_constants::{
    OTHER_REPOSITORY_TEST_SOURCE, REPOSITORY_SUPPORT_SOURCE, REPOSITORY_TEST_SOURCE,
};
use crate::{coverage_data, coverage_json, declaration_only};
use serde_json::json;
use std::path::Path;
use yolo_coverage_source_contract::SourceContract;

const EXECUTION_ROOT: &str = "/workspace";
const CORE_SOURCE: &str = "core/crates/wire/src/lib.rs";
const TOOLING_SOURCE: &str = "build_system/architecture/checks/ownership/src/test_ownership.rs";
const UNIT_TEST_SOURCE: &str = "build_system/tool/unit_tests/case.rs";
const SUPPORT_SOURCE: &str = "build_system/tool_test_support/src/lib.rs";

#[test]
fn llvm_summaries_include_production_tooling_policies_but_exclude_tool_tests() {
    let files = [
        CORE_SOURCE,
        TOOLING_SOURCE,
        UNIT_TEST_SOURCE,
        SUPPORT_SOURCE,
    ]
    .map(|source| {
        let metric = json!({"count": 1, "covered": 1});
        json!({
            "filename": format!("{EXECUTION_ROOT}/{source}"),
            "summary": {
                "lines": metric, "functions": metric,
                "regions": metric, "branches": metric,
            },
        })
    });
    let report = json!({"data": [{"files": files, "functions": []}]}).to_string();
    let data = coverage_json::parse(&report).unwrap();
    let summaries = coverage_data::summaries(&data, Path::new(EXECUTION_ROOT)).unwrap();
    assert_eq!(
        coverage_data::sources(&summaries),
        [TOOLING_SOURCE, CORE_SOURCE]
    );
}

#[test]
fn repository_tests_and_fixture_mappings_are_admitted_only_from_the_declared_manifest() {
    let manifest = format!("{REPOSITORY_TEST_SOURCE}\n{REPOSITORY_SUPPORT_SOURCE}");
    for prefix in ["", BAZEL_WITNESS_PREFIX] {
        let mut observed = data_for(&format!("{prefix}{REPOSITORY_TEST_SOURCE}"));
        let helper = data_for(&format!("{prefix}{REPOSITORY_SUPPORT_SOURCE}"));
        observed.files.extend(helper.files);
        observed.functions.extend(helper.functions);
        assert_eq!(
            declaration_only::validate(&observed, Path::new(ROOT), &manifest, read).unwrap(),
            SourceContract::TestsOnly
        );
    }
}

#[test]
fn repository_fixtures_and_unexecuted_tests_cannot_stand_in_for_executed_test_evidence() {
    let manifest = format!("{REPOSITORY_TEST_SOURCE}\n{REPOSITORY_SUPPORT_SOURCE}");
    let mutations: &[fn(&mut Data)] = &[
        |data| data.files.clear(),
        |data| data.functions.clear(),
        |data| data.functions[FILE_INDEX as usize].count = 0,
        |data| data.functions[FILE_INDEX as usize].regions.clear(),
        |data| data.files[FILE_INDEX as usize].summary.functions.covered = 0,
        |data| data.files[FILE_INDEX as usize].summary.regions.covered = u64::MAX,
    ];
    for mutate in mutations {
        let mut observed = data_for(REPOSITORY_TEST_SOURCE);
        mutate(&mut observed);
        assert!(declaration_only::validate(&observed, Path::new(ROOT), &manifest, read).is_err());
    }
    let helper_only = data_for(REPOSITORY_SUPPORT_SOURCE);
    assert!(declaration_only::validate(&helper_only, Path::new(ROOT), &manifest, read).is_err());
}

#[test]
fn repository_contracts_cannot_hide_undeclared_or_production_file_and_function_mappings() {
    let manifest = format!("{REPOSITORY_TEST_SOURCE}\n{REPOSITORY_SUPPORT_SOURCE}");
    for source in [SOURCE, OTHER_REPOSITORY_TEST_SOURCE] {
        let mut observed = data_for(REPOSITORY_TEST_SOURCE);
        observed.files[FILE_INDEX as usize].filename = source.into();
        assert!(declaration_only::validate(&observed, Path::new(ROOT), &manifest, read).is_err());
        let mut observed = data_for(REPOSITORY_TEST_SOURCE);
        observed.functions[FILE_INDEX as usize]
            .filenames
            .push(source.into());
        assert!(declaration_only::validate(&observed, Path::new(ROOT), &manifest, read).is_err());
    }
    for incomplete in ["", REPOSITORY_SUPPORT_SOURCE] {
        assert!(
            declaration_only::validate(
                &data_for(REPOSITORY_TEST_SOURCE),
                Path::new(ROOT),
                incomplete,
                read,
            )
            .is_err()
        );
    }
}
