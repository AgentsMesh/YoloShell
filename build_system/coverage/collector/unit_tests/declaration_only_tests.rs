use crate::constants::DECLARATION_FILE_LIMIT;
use crate::coverage_json::Data;
use crate::declaration_fixture::{data, manifest, read};
use crate::instance_fixture::{
    BAZEL_WITNESS_PREFIX, FILE_INDEX, ROOT, SOURCE, TEST_SOURCE, TEST_WORKSPACE,
};
use crate::test_constants::{OTHER_TEST_SOURCE, TEST_SUPPORT_SOURCE};
use crate::{declaration_only, declaration_sources};
use std::path::Path;
use yolo_coverage_source_contract::DECLARATION_SOURCE_SETS;

#[test]
fn reviewed_and_test_only_contracts_still_require_executed_llvm_mappings() {
    for manifest in DECLARATION_SOURCE_SETS
        .iter()
        .map(|set| manifest(set))
        .chain([format!("{TEST_SOURCE}\n{TEST_SUPPORT_SOURCE}")])
    {
        declaration_only::validate(&data(), Path::new(ROOT), &manifest, read).unwrap();
        declaration_only::validate(
            &data(),
            Path::new(ROOT),
            &manifest.replace('\n', "\r\n"),
            read,
        )
        .unwrap();
    }
}

#[test]
fn bazel_compiled_test_paths_still_require_declared_test_and_source_evidence() {
    for manifest in [
        manifest(DECLARATION_SOURCE_SETS[FILE_INDEX as usize]),
        TEST_SOURCE.to_owned(),
    ] {
        let mut observed = data();
        let mapped = format!("{BAZEL_WITNESS_PREFIX}{TEST_SOURCE}");
        observed.files[FILE_INDEX as usize].filename = mapped.clone();
        observed.functions[FILE_INDEX as usize].filenames[FILE_INDEX as usize] = mapped;
        declaration_only::validate(&observed, Path::new(ROOT), &manifest, read).unwrap();
        observed.functions[FILE_INDEX as usize].filenames[FILE_INDEX as usize] =
            format!("{BAZEL_WITNESS_PREFIX}{SOURCE}");
        assert!(declaration_only::validate(&observed, Path::new(ROOT), &manifest, read).is_err());
        for invalid in [
            "bazel-out/../bin/",
            "bazel-out/config/elsewhere/",
            "bazel-out/config/bin/../",
        ] {
            let mapped = format!("{invalid}{TEST_SOURCE}");
            observed.files[FILE_INDEX as usize].filename = mapped.clone();
            observed.functions[FILE_INDEX as usize].filenames[FILE_INDEX as usize] = mapped;
            assert!(
                declaration_only::validate(&observed, Path::new(ROOT), &manifest, read).is_err()
            );
        }
    }
}

#[test]
fn ordinary_production_mappings_cannot_use_declaration_only_admission() {
    let sources = DECLARATION_SOURCE_SETS[FILE_INDEX as usize];
    let mut data = data();
    data.files[FILE_INDEX as usize].filename = SOURCE.to_owned();
    assert!(declaration_only::has_production_files(&data, Path::new(ROOT)).unwrap());
    assert!(declaration_only::validate(&data, Path::new(ROOT), &manifest(sources), read).is_err());
    data.files[FILE_INDEX as usize].filename = TEST_SOURCE.to_owned();
    data.functions[FILE_INDEX as usize].filenames[FILE_INDEX as usize] = SOURCE.to_owned();
    assert!(declaration_only::validate(&data, Path::new(ROOT), &manifest(sources), read).is_err());
}

#[test]
fn empty_missing_or_unexecuted_instrumentation_is_not_an_empty_code_exception() {
    let mutations: &[fn(&mut Data)] = &[
        |data| data.files.clear(),
        |data| data.functions.clear(),
        |data| data.functions[FILE_INDEX as usize].count = 0,
        |data| data.functions[FILE_INDEX as usize].name.clear(),
        |data| data.functions[FILE_INDEX as usize].regions.clear(),
        |data| data.files[FILE_INDEX as usize].summary.functions.covered = 0,
        |data| data.files[FILE_INDEX as usize].summary.regions.covered = u64::MAX,
        |data| data.functions[FILE_INDEX as usize].filenames[FILE_INDEX as usize] = ROOT.to_owned(),
    ];
    for manifest in [
        manifest(DECLARATION_SOURCE_SETS[FILE_INDEX as usize]),
        format!("{TEST_SOURCE}\n{TEST_SUPPORT_SOURCE}"),
    ] {
        for mutate in mutations {
            let mut observed = data();
            mutate(&mut observed);
            assert!(
                declaration_only::validate(&observed, Path::new(ROOT), &manifest, read).is_err()
            );
        }
        let absent_test = manifest.replace(TEST_SOURCE, "");
        assert!(declaration_only::validate(&data(), Path::new(ROOT), &absent_test, read).is_err());
        let mut observed = data();
        observed.files[FILE_INDEX as usize].filename = TEST_SUPPORT_SOURCE.into();
        observed.functions[FILE_INDEX as usize].filenames[FILE_INDEX as usize] =
            TEST_SUPPORT_SOURCE.into();
        assert!(declaration_only::validate(&observed, Path::new(ROOT), &manifest, read).is_err());
    }
}

#[test]
fn llvm_cannot_add_unmanifested_tests_or_production_files_and_functions() {
    for source in [SOURCE, OTHER_TEST_SOURCE, TEST_SUPPORT_SOURCE, "unknown.rs"] {
        let mut observed = data();
        observed.files[FILE_INDEX as usize].filename = source.into();
        assert!(declaration_only::validate(&observed, Path::new(ROOT), TEST_SOURCE, read).is_err());
        let mut observed = data();
        observed.functions[FILE_INDEX as usize]
            .filenames
            .push(source.into());
        assert!(declaration_only::validate(&observed, Path::new(ROOT), TEST_SOURCE, read).is_err());
    }
}

#[test]
fn evidence_reads_are_bounded_and_missing_files_are_rejected() {
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().set_len(DECLARATION_FILE_LIMIT + 1).unwrap();
    assert!(declaration_sources::read_bounded(file.path()).is_err());
    assert!(declaration_sources::read_bounded(&file.path().join(SOURCE)).is_err());
}

#[test]
fn runfile_evidence_uses_the_declared_workspace_and_rejects_traversal() {
    let execution = Path::new(ROOT);
    let relative = Path::new("test.runfiles");
    let workspace = Path::new(TEST_WORKSPACE);
    assert_eq!(
        declaration_sources::runfile_root(execution, relative, workspace).unwrap(),
        execution.join(relative).join(workspace)
    );
    assert_eq!(
        declaration_sources::runfile_root(execution, execution, workspace).unwrap(),
        execution.join(workspace)
    );
    for invalid in ["", ".", "..", "../other", "/other", "nested/other"] {
        assert!(
            declaration_sources::runfile_root(execution, relative, Path::new(invalid)).is_err()
        );
    }
}
