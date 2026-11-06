use crate::constants::{BAZEL_WITNESS_PREFIX, SOURCE};
use yolo_coverage_source_contract::reject_staged_production;

const UNREMAPPED: &str = "unremapped declared production coverage source";
const LOAD_ERROR: &str = "manifest read failed";
const OTHER_SOURCE: &str = "build_system/fixture/src/other.rs";

#[test]
fn every_declared_staged_candidate_is_rejected_without_requiring_execution_counts() {
    for source in [
        SOURCE,
        "core/crates/transport/local/unix/src/async_stream.rs",
    ] {
        let paths = [format!("{BAZEL_WITNESS_PREFIX}{source}")];
        let error = reject_staged_production(paths, || Ok(source.into())).unwrap_err();
        assert!(error.contains(UNREMAPPED));
        assert!(error.contains(source));
    }
}

#[test]
fn canonical_dependencies_and_duplicate_paths_cannot_hide_a_staged_candidate() {
    for source in [SOURCE, OTHER_SOURCE] {
        let staged = format!("{BAZEL_WITNESS_PREFIX}{source}");
        let paths = [SOURCE, staged.as_str(), staged.as_str()];
        let manifest = if source == SOURCE {
            SOURCE.into()
        } else {
            format!("{SOURCE}\n{source}")
        };
        assert!(
            reject_staged_production(paths, || Ok(manifest))
                .unwrap_err()
                .contains(UNREMAPPED)
        );
    }
}

#[test]
fn canonical_generated_test_and_external_paths_never_load_the_manifest() {
    let mut paths = vec![SOURCE.to_owned()];
    for source in [
        "core/crates/example/src/generated/contract.rs",
        "core/crates/example/unit_tests/case.rs",
        "build_system/example/test_support/src/helper.rs",
        "external/dependency/src/lib.rs",
        "core/crates/example/source.rs",
        "core/crates/example/src/data.txt",
    ] {
        paths.push(format!("{BAZEL_WITNESS_PREFIX}{source}"));
    }
    reject_staged_production(paths, || panic!("unexpected manifest load")).unwrap();
    reject_staged_production(std::iter::empty::<&str>(), || {
        panic!("unexpected manifest load")
    })
    .unwrap();
}

#[test]
fn a_staged_candidate_requires_valid_manifest_without_promoting_undeclared_files() {
    let path = format!("{BAZEL_WITNESS_PREFIX}{SOURCE}");
    assert_eq!(
        reject_staged_production([&path], || Err(LOAD_ERROR.into())),
        Err(LOAD_ERROR.into())
    );
    for manifest in ["", "not-a-source", "../source.rs", "/source.rs", "bad\0.rs"] {
        assert!(reject_staged_production([&path], || Ok(manifest.into())).is_err());
    }
    reject_staged_production([&path], || Ok(OTHER_SOURCE.into())).unwrap();
}
