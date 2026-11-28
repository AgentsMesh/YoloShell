use crate::linux_runtime_output::capture_script;
use std::path::Path;
use yolo_release_contracts::*;

#[path = "../test_support/linux_runtime_output_fixture.rs"]
mod fixture;
use fixture::*;

#[test]
fn configured_alias_resolves_the_crate_executable_not_a_stale_package_path() {
    let fixture = Fixture::new();
    fixture.executable(ACTUAL_OUTPUT);
    fixture.executable(STALE_OUTPUT);
    let result = fixture.capture(ACTUAL_OUTPUT, QUERY_SUCCESS);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let selected = String::from_utf8(result.stdout).unwrap();
    assert_eq!(Path::new(&selected), fixture.link_path(ACTUAL_OUTPUT));
    assert_ne!(Path::new(&selected), fixture.link_path(STALE_OUTPUT));
}

#[test]
fn query_failure_cannot_reuse_the_existing_old_location() {
    let fixture = Fixture::new();
    fixture.executable(STALE_OUTPUT);
    fixture.executable(ACTUAL_OUTPUT);
    assert!(
        !fixture
            .capture(ACTUAL_OUTPUT, QUERY_FAILURE)
            .status
            .success()
    );
}

#[test]
fn missing_non_executable_and_symlink_outputs_are_rejected() {
    let fixture = Fixture::new();
    assert!(
        !fixture
            .capture(ACTUAL_OUTPUT, QUERY_SUCCESS)
            .status
            .success()
    );
    fixture.executable(ACTUAL_OUTPUT);
    fixture.remove_execute_permission(ACTUAL_OUTPUT);
    assert!(
        !fixture
            .capture(ACTUAL_OUTPUT, QUERY_SUCCESS)
            .status
            .success()
    );
    fixture.replace_with_symlink(ACTUAL_OUTPUT, STALE_OUTPUT);
    assert!(
        !fixture
            .capture(ACTUAL_OUTPUT, QUERY_SUCCESS)
            .status
            .success()
    );
}

#[test]
fn ambiguous_or_escaping_query_output_is_rejected() {
    let fixture = Fixture::new();
    fixture.executable(ACTUAL_OUTPUT);
    for output in [
        String::new(),
        format!("{ACTUAL_OUTPUT}\n{ACTUAL_OUTPUT}"),
        format!("{ACTUAL_OUTPUT}\r"),
        format!("/{ACTUAL_OUTPUT}"),
        "bazel-out/../outside".to_owned(),
        "bazel-out/./outside".to_owned(),
        "bazel-out//outside".to_owned(),
        "source/core/yolo".to_owned(),
    ] {
        assert!(
            !fixture.capture(&output, QUERY_SUCCESS).status.success(),
            "{output:?}"
        );
    }
}
