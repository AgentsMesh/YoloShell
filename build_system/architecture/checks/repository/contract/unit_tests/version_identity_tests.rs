use yolo_architecture_test_support::Fixture;

const VERSION_PATH: &str = "build_system/bazel/version.bzl";
const SNAPSHOT_PATH: &str = "build_system/release/contracts/src/snapshot.rs";
const LINUX_PATH: &str = "build_system/release/contracts/src/linux.rs";

fn messages(snapshot: &str, linux: Option<&str>) -> Vec<String> {
    let fixture = Fixture::new();
    fixture.write(VERSION_PATH, "YOLO_MARKETING_VERSION = \"0.1.0\"\n");
    fixture.write(SNAPSHOT_PATH, snapshot);
    if let Some(linux) = linux {
        fixture.write(LINUX_PATH, linux);
    }
    let files = yolo_architecture_test_support::load(fixture.root()).unwrap();
    let mut violations = Vec::new();
    crate::version_identity_rules::inspect(&files, &mut violations);
    violations
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

const MATCHING: &str = "pub const YOLO_MARKETING_VERSION: &str = \"0.1.0\";\n";
const ALIASED: &str = "pub use crate::snapshot::YOLO_MARKETING_VERSION as LINUX_RUNTIME_VERSION;\n";

#[test]
fn accepts_matching_bazel_and_release_versions() {
    let report = messages(MATCHING, Some(ALIASED));

    assert!(report.is_empty(), "{report:?}");
}

#[test]
fn rejects_a_version_mismatch() {
    let report = messages(
        "pub const YOLO_MARKETING_VERSION: &str = \"0.2.0\";\n",
        None,
    );

    assert_eq!(report.len(), 1, "{report:?}");
    assert!(report[0].contains("must match"), "{report:?}");
}

#[test]
fn rejects_a_missing_release_version() {
    let report = messages("pub const OTHER: &str = \"0.1.0\";\n", None);

    assert_eq!(report.len(), 1, "{report:?}");
    assert!(report[0].contains("must be declared"), "{report:?}");
}

#[test]
fn rejects_a_redeclared_linux_version() {
    let linux = "pub const LINUX_RUNTIME_VERSION: &str = \"0.1.0\";\n";
    let report = messages(MATCHING, Some(linux));

    assert_eq!(report.len(), 1, "{report:?}");
    assert!(report[0].contains("must not redeclare"), "{report:?}");
}
