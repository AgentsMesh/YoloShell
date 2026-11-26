use crate::declaration_fixture::{data, data_for, read};
use crate::declaration_only;
use crate::instance_fixture::{FILE_INDEX, ROOT, SOURCE, TEST_SOURCE};
use std::path::Path;

const EXTERNAL_SOURCE: &str = "external/rules_rust++crate+crates__serde_json-1.0.151/src/error.rs";
const UNKNOWN_SOURCE: &str = "unknown.rs";
const UNDECLARED_SOURCE: &str = "build_system/fixture/unit_tests/undeclared.rs";

#[test]
fn filtered_dependency_function_mappings_do_not_become_owned_test_evidence() {
    for source in [
        EXTERNAL_SOURCE.to_owned(),
        format!("./{EXTERNAL_SOURCE}"),
        format!("{ROOT}/{EXTERNAL_SOURCE}"),
        format!("#/proc/self/cwd/{EXTERNAL_SOURCE}"),
    ] {
        let mut observed = data();
        observed.functions.extend(data_for(&source).functions);
        declaration_only::validate(&observed, Path::new(ROOT), TEST_SOURCE, read).unwrap();
        observed.functions.remove(FILE_INDEX as usize);
        assert!(declaration_only::validate(&observed, Path::new(ROOT), TEST_SOURCE, read).is_err());
    }
}

#[test]
fn external_file_summaries_are_not_hidden_and_mixed_function_paths_remain_checked() {
    let mut observed = data();
    observed.files.extend(data_for(EXTERNAL_SOURCE).files);
    assert!(declaration_only::validate(&observed, Path::new(ROOT), TEST_SOURCE, read).is_err());
    for unknown in [UNKNOWN_SOURCE, UNDECLARED_SOURCE, SOURCE] {
        let mut observed = data();
        let mut external = data_for(EXTERNAL_SOURCE)
            .functions
            .remove(FILE_INDEX as usize);
        external.filenames.push(unknown.into());
        observed.functions.push(external);
        let error =
            declaration_only::validate(&observed, Path::new(ROOT), TEST_SOURCE, read).unwrap_err();
        assert!(error.contains(unknown), "{error}");
    }
}

#[test]
fn external_functions_cannot_borrow_a_test_filename_without_executing_its_region() {
    let mut observed = data();
    observed.functions = data_for(EXTERNAL_SOURCE).functions;
    observed.functions[FILE_INDEX as usize]
        .filenames
        .push(TEST_SOURCE.into());
    assert!(declaration_only::validate(&observed, Path::new(ROOT), TEST_SOURCE, read).is_err());
    // A real test function may reference an external macro mapping as well.
    let mut observed = data();
    observed.functions[FILE_INDEX as usize]
        .filenames
        .push(EXTERNAL_SOURCE.into());
    declaration_only::validate(&observed, Path::new(ROOT), TEST_SOURCE, read).unwrap();
    for change in [
        crate::coverage_json::COUNT_INDEX,
        crate::coverage_json::KIND_INDEX,
    ] {
        let mut observed = data();
        observed.functions[FILE_INDEX as usize].regions[FILE_INDEX as usize][change] =
            if change == crate::coverage_json::COUNT_INDEX {
                0
            } else {
                crate::coverage_json::EXPANSION_REGION_KIND
            };
        assert!(declaration_only::validate(&observed, Path::new(ROOT), TEST_SOURCE, read).is_err());
    }
}

#[test]
fn dependency_exclusion_requires_a_normalized_execution_owned_external_source() {
    for source in [
        "external.rs",
        "external/dependency.rs",
        "external/dependency/source.c",
        "external/dependency/../source.rs",
        "external/dependency/./source.rs",
        "external//dependency/source.rs",
        "external/dependency/source.rs/",
        "external/dependency/\0source.rs",
        "outside/external/dependency/source.rs",
        "external_suffix/dependency/source.rs",
        "core/crates/external/src/source.rs",
        "/external/dependency/source.rs",
        "/workspace-other/external/dependency/source.rs",
        "bazel-out/config/bin/external/dependency/source.rs",
    ] {
        let mut observed = data();
        observed.functions.extend(data_for(source).functions);
        assert!(
            declaration_only::validate(&observed, Path::new(ROOT), TEST_SOURCE, read).is_err(),
            "{source:?}"
        );
    }
}
