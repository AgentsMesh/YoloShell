use crate::{constants::*, fixture::read};
use yolo_coverage_source_contract::{SourceContract, verify_manifest};

#[test]
fn test_only_manifest_does_not_invent_reviewed_production_fingerprints() {
    let manifest = [TEST_SOURCE, TEST_SUPPORT_SOURCE]
        .into_iter()
        .chain(TEST_ONLY_ARTIFACTS.iter().copied())
        .collect::<Vec<_>>()
        .join("\n");
    for value in [manifest.clone(), manifest.replace('\n', "\r\n")] {
        assert_eq!(
            verify_manifest(&value, |_| {
                panic!("test-only collection must not invent production source fingerprints")
            })
            .unwrap(),
            SourceContract::TestsOnly
        );
    }
}

#[test]
fn missing_production_and_unknown_sources_cannot_be_reclassified_as_test_only() {
    for source in [
        SOURCE,
        "build_system/fixture/runtime.rs",
        "core/crates/fixture/src/lib.rs",
        "plugins/tui/fixture/src/lib.rs",
        "plugins/web/gateway/src/lib.rs",
        "outside/unit_tests/case.rs",
        "external/crate/unit_tests/case.rs",
        "build_system/fixture/fixtures/src/lib.rs",
        "build_system/fixture/unit_tests_suffix/case.rs",
        "build_system/fixture/test_supporting/parser.rs",
        "build_system/fixture/unit_tests/../src/lib.rs",
        "build_system/fixture/unit_tests//case.rs",
        "bazel-out/test-config/bin/build_system/fixture/src/lib.rs",
        "external/toolchain/src/runtime.c",
        "external/unknown-tool",
        "external/unknown-library.so",
        "bazel-out/unknown-artifact",
        "unknown-artifact",
        "/workspace/build_system/fixture/unit_tests/case.rs",
    ] {
        let manifest = format!("{TEST_SOURCE}\n{source}");
        assert!(verify_manifest(&manifest, read).is_err(), "{source}");
    }
}

#[test]
fn empty_support_only_and_duplicate_source_manifests_are_not_test_evidence() {
    for manifest in [
        String::new(),
        TEST_SUPPORT_SOURCE.to_owned(),
        TEST_ONLY_ARTIFACTS.join("\n"),
        format!("{TEST_SOURCE}\n{TEST_SOURCE}"),
        format!("{TEST_SOURCE}\n{TEST_SUPPORT_SOURCE}\n{TEST_SUPPORT_SOURCE}"),
    ] {
        assert!(verify_manifest(&manifest, read).is_err());
    }
}

#[test]
fn ancillary_entries_require_complete_normalized_output_paths_without_source_extensions() {
    for artifact in [
        "",
        "/",
        "./",
        "bazel-out",
        "bazel-out/test-config/bin",
        "bazel-out/test-config/../bin/probe",
        "external/toolchain/../llvm/bin/llvm-cov",
        "bazel-out/test-config/bin/probe.c",
        "bazel-out/test-config/bin/probe.profile",
    ] {
        let manifest = format!("{artifact}\n{TEST_SOURCE}");
        assert!(verify_manifest(&manifest, read).is_err(), "{artifact}");
    }
}
