use super::super::super::runtime_record_test_support::{bind_record, record_for_identity};
use super::*;

const CUSTODY_FIXTURE_KEY: &str = "custody";
const CUSTODY_FIXTURE_VALUE: &str = "unexpected";

#[test]
fn explicit_absence_accepts_a_fixture_without_custody() {
    let (_directory, paths, bound) = fixture();

    assert!(
        capture(
            &paths,
            std::slice::from_ref(&bound),
            GuardianExpectation::ExplicitlyAbsentFixture,
        )
        .unwrap()
        .is_empty()
    );
}

#[test]
fn explicit_absence_rejects_a_structured_custody_artifact() {
    let (_directory, paths, bound) = fixture();
    let path = paths.pty_custody(bound.record().terminal_id);
    let custody = serde_json::json!({CUSTODY_FIXTURE_KEY: CUSTODY_FIXTURE_VALUE});
    yolo_runtime::paths::atomic_write_json(&path, &custody).unwrap();

    assert!(
        capture(
            &paths,
            std::slice::from_ref(&bound),
            GuardianExpectation::ExplicitlyAbsentFixture,
        )
        .is_err()
    );
}

#[test]
fn supported_only_expectation_fails_closed_on_an_unsupported_platform() {
    let (_directory, paths, bound) = fixture();

    assert!(
        capture(
            &paths,
            std::slice::from_ref(&bound),
            GuardianExpectation::RequiredOnSupportedPlatform,
        )
        .is_err()
    );
}

fn fixture() -> (tempfile::TempDir, Paths, BoundRuntimeRecord) {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    let bound = bind_record(&paths, record);
    (directory, paths, bound)
}
