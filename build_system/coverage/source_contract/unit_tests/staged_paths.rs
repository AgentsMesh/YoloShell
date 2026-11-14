use crate::constants::{BAZEL_WITNESS_PREFIX, SOURCE};
use yolo_coverage_source_contract::reject_staged_production;

const INVALID_STAGING: &str = "invalid staged coverage source path";
const INVALID_MANIFEST: &str = "invalid staged coverage source manifest";
const UNREMAPPED: &str = "unremapped declared production coverage source";

#[test]
fn staged_aliases_require_an_exact_normalized_bazel_bin_layout() {
    for path in [
        "bazel-out",
        "bazel-out/",
        "bazel-out/config",
        "bazel-out/config/bin",
        "bazel-out/config/bin/",
        "bazel-out/../bin/core/src/lib.rs",
        "bazel-out//bin/core/src/lib.rs",
        "bazel-out/config/elsewhere/core/src/lib.rs",
        "bazel-out/config/bin/../core/src/lib.rs",
        "bazel-out/config/bin/./core/src/lib.rs",
        "bazel-out/config/bin/core/src/bad\0.rs",
    ] {
        assert_eq!(
            reject_staged_production([path], || panic!("invalid path cannot load manifest")),
            Err(INVALID_STAGING.into()),
            "{path}"
        );
    }
    reject_staged_production(["other/bazel-out/config/bin/core/src/lib.rs"], || {
        panic!("unrelated path cannot load manifest")
    })
    .unwrap();
}

#[test]
fn declared_handwritten_sources_are_exact_and_manifest_paths_are_validated() {
    let path = format!("{BAZEL_WITNESS_PREFIX}{SOURCE}");
    let manifest = format!(
        "instrumented-binary\n{SOURCE}\r\ncore/src/lib.rs\ncore/src/generated/contract.rs\n\
         core/crates/example/unit_tests/case.rs\nexternal/dependency/src/lib.rs\n\
         core/crates/example/source.rs\ncore/crates/example/src/data.txt\n"
    );
    assert!(
        reject_staged_production([&path], || Ok(manifest))
            .unwrap_err()
            .contains(UNREMAPPED)
    );
    for manifest in [
        format!("{SOURCE}\n{SOURCE}"),
        format!("./{SOURCE}"),
        format!("{SOURCE}\ninvalid\0entry"),
    ] {
        assert_eq!(
            reject_staged_production([&path], || Ok(manifest)),
            Err(INVALID_MANIFEST.into())
        );
    }
}
