use crate::{constants::*, fixture::read};
use yolo_coverage_protocol::production_key;
use yolo_coverage_source_contract::{SourceContract, is_test_source, verify_manifest};

#[test]
fn repository_fixture_sources_need_an_explicit_test_and_never_change_production_scope() {
    assert!(is_test_source(REPOSITORY_UNIT_SOURCE, false).unwrap());
    assert!(is_test_source(REPOSITORY_FIXTURE_SOURCE, true).unwrap());
    assert!(!is_test_source(REPOSITORY_FIXTURE_SOURCE, false).unwrap());
    for source in [REPOSITORY_UNIT_SOURCE, REPOSITORY_FIXTURE_SOURCE] {
        assert_eq!(production_key(source).unwrap(), None);
    }
    let manifest = [REPOSITORY_UNIT_SOURCE, REPOSITORY_FIXTURE_SOURCE]
        .into_iter()
        .chain(TEST_ONLY_ARTIFACTS.iter().copied())
        .collect::<Vec<_>>()
        .join("\n");
    for value in [manifest.clone(), manifest.replace('\n', "\r\n")] {
        assert_eq!(
            verify_manifest(&value, |_| panic!(
                "test inputs have no production fingerprint"
            ))
            .unwrap(),
            SourceContract::TestsOnly
        );
    }
    for manifest in [
        REPOSITORY_FIXTURE_SOURCE.to_owned(),
        format!("{REPOSITORY_UNIT_SOURCE}\n{REPOSITORY_UNIT_SOURCE}"),
        format!("{REPOSITORY_UNIT_SOURCE}\n{SOURCE}"),
        format!("{REPOSITORY_UNIT_SOURCE}\ncore/crates/fixture/src/lib.rs"),
    ] {
        assert!(verify_manifest(&manifest, read).is_err());
    }
}

#[test]
fn repository_test_paths_cannot_escape_or_normalize_away_invalid_components() {
    for source in [
        "integration_tests/runtime/../unit_tests/case.rs",
        "integration_tests/runtime/./unit_tests/case.rs",
        "integration_tests/runtime//unit_tests/case.rs",
        "integration_tests/runtime/unit_tests/\0case.rs",
    ] {
        assert!(is_test_source(source, true).is_err(), "{source:?}");
        assert!(is_test_source(source, false).is_err(), "{source:?}");
        let manifest = format!("{REPOSITORY_UNIT_SOURCE}\n{source}");
        assert!(verify_manifest(&manifest, read).is_err(), "{source:?}");
    }
    for source in [
        "integration_tests_suffix/runtime/unit_tests/case.rs",
        "outside/integration_tests/runtime/unit_tests/case.rs",
        "external/integration_tests/runtime/unit_tests/case.rs",
        "/integration_tests/runtime/unit_tests/case.rs",
        "./integration_tests/runtime/unit_tests/case.rs",
        "integration_tests/runtime/unit_tests/case.txt",
        "integration_tests/runtime/unit_tests/case",
    ] {
        assert!(!is_test_source(source, true).unwrap(), "{source:?}");
        let manifest = format!("{REPOSITORY_UNIT_SOURCE}\n{source}");
        assert!(verify_manifest(&manifest, read).is_err(), "{source:?}");
    }
}

#[test]
fn fixture_names_and_test_directory_lookalikes_do_not_witness_executed_tests() {
    for source in [
        REPOSITORY_FIXTURE_SOURCE,
        "integration_tests/runtime/unit_tests_suffix/case.rs",
        "integration_tests/runtime/test_support/parser.rs",
        "integration_tests/runtime/unit_tests.rs",
    ] {
        assert!(is_test_source(source, true).unwrap());
        assert!(!is_test_source(source, false).unwrap());
        assert!(verify_manifest(source, read).is_err());
    }
}
