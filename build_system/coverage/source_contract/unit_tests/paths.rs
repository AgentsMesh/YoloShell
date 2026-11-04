use crate::constants::*;
use std::path::Path;
use yolo_coverage_source_contract::{
    SourceContract, is_external_rust_source, is_test_source, mapped_source, verify_manifest,
};

const EXECUTION_ROOT: &str = "/workspace";
const EXTERNAL_SOURCE: &str = "external/rules_rust++crate+crates__serde_json-1.0.151/src/error.rs";

#[test]
fn external_rust_sources_require_exact_workspace_or_known_raw_prefixes() {
    for source in [
        EXTERNAL_SOURCE.to_owned(),
        format!("./{EXTERNAL_SOURCE}"),
        format!("{EXECUTION_ROOT}/{EXTERNAL_SOURCE}"),
        format!("#/proc/self/cwd/{EXTERNAL_SOURCE}"),
    ] {
        assert!(
            is_external_rust_source(&source, Path::new(EXECUTION_ROOT)),
            "{source:?}"
        );
    }
}

#[test]
fn external_classification_cannot_hide_unknown_or_malformed_repository_paths() {
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
        assert!(
            !is_external_rust_source(source, Path::new(EXECUTION_ROOT)),
            "{source:?}"
        );
    }
}

#[test]
fn mapped_bazel_paths_recover_source_identity_only_for_the_known_output_layout() {
    assert_eq!(
        mapped_source(&format!("{BAZEL_WITNESS_PREFIX}{TEST_SOURCE}")),
        TEST_SOURCE
    );
    assert_eq!(mapped_source(TEST_SOURCE), TEST_SOURCE);
    for prefix in [
        "bazel-out/../bin/",
        "bazel-out/config/elsewhere/",
        "bazel-out/config/bin/../",
    ] {
        let source = format!("{prefix}{TEST_SOURCE}");
        assert_eq!(mapped_source(&source), source);
    }
}

#[test]
fn test_source_identity_requires_the_protocol_scope_and_an_exact_test_directory() {
    assert!(is_test_source(TEST_SOURCE, false).unwrap());
    assert!(is_test_source(TEST_SUPPORT_SOURCE, true).unwrap());
    assert!(!is_test_source(TEST_SUPPORT_SOURCE, false).unwrap());
    for source in [
        SOURCE,
        "outside/unit_tests/case.rs",
        "build_system/fixture/unit_tests/case.txt",
    ] {
        assert!(!is_test_source(source, true).unwrap(), "{source}");
    }
    assert!(is_test_source("core/src/../unit_tests/case.rs", true).is_err());
}

#[test]
fn every_supported_repository_root_admits_tests_and_their_support_sources() {
    for root in REPOSITORY_TEST_ROOTS {
        let unit = format!("{root}unit_tests/case.rs");
        let integration = format!("{root}integration_tests/case.rs");
        let support = format!("{root}test_support/parser.rs");
        assert!(is_test_source(&unit, false).unwrap());
        assert!(is_test_source(&integration, false).unwrap());
        assert!(is_test_source(&support, true).unwrap());
        assert!(!is_test_source(&support, false).unwrap());
        let manifest = format!("{unit}\n{integration}\n{support}");
        assert_eq!(
            verify_manifest(&manifest, |_| {
                panic!("test sources must not request production declaration bytes")
            })
            .unwrap(),
            SourceContract::TestsOnly
        );
    }
}

#[test]
fn a_test_directory_does_not_turn_an_extensionless_path_into_rust_source_evidence() {
    for root in REPOSITORY_TEST_ROOTS {
        for relative in [
            "unit_tests/case",
            EXTENSIONLESS_INTEGRATION_DIRECTORY,
            "test_support/parser",
        ] {
            let source = format!("{root}{relative}");
            assert!(!is_test_source(&source, true).unwrap(), "{source}");
            let manifest = format!("{TEST_SOURCE}\n{source}");
            assert!(
                verify_manifest(&manifest, crate::fixture::read).is_err(),
                "{source}"
            );
        }
    }
}
