use crate::constants::*;
use crate::entrypoint_fixture::*;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

#[test]
fn collector_child() {
    if std::env::var(FIXTURE_CHILD_ENV).as_deref() == Ok(CHILD_TEST) {
        if std::env::var(FIXTURE_FAILURE_ENV).as_deref() == Ok(ENTRY_READ_FAILURE) {
            crate::run_with_context(partially_unreadable_inventory);
        } else {
            crate::run();
        }
    }
}

#[test]
fn successful_pipeline_resolves_tools_and_objects_and_preserves_foreign_markers() {
    for runfiles_tool in [false, true] {
        for absolute_runfiles in [false, true] {
            let mut fixture = Fixture::new();
            if runfiles_tool {
                std::fs::remove_file(fixture.root().join(FIXTURE_TOOL)).unwrap();
            }
            if absolute_runfiles {
                fixture.command.env(
                    RUNFILES_DIRECTORY_ENV,
                    fixture.root().join(FIXTURE_RUNFILES),
                );
            }
            fixture
                .command
                .env(VERBOSE_ENV, "1")
                .env(INSTRUMENTED_TEST_BINARY_ENV, FIXTURE_BINARY)
                .env(
                    ADDITIONAL_COVERAGE_OBJECTS_ENV,
                    serde_json::to_string(&[FIXTURE_ADDITIONAL]).unwrap(),
                );
            std::fs::write(fixture.marker(), FIXTURE_MARKER).unwrap();
            fixture.succeeds();
            assert_eq!(
                std::fs::read_to_string(fixture.marker()).unwrap(),
                FIXTURE_MARKER
            );
        }
    }
    let mut fixture = Fixture::new();
    fixture.succeeds();
    assert!(!fixture.marker().exists());
}

#[test]
fn failed_llvm_stages_and_invalid_reports_retain_incomplete_marker() {
    for (stage, error) in [
        (MERGE_STAGE, "llvm-profdata failed"),
        (LCOV_STAGE, "llvm-cov failed"),
        (JSON_STAGE, "llvm-cov failed"),
        (REPORT_STAGE, "llvm-cov failed"),
        (UTF8_FAILURE, "output is not UTF-8"),
        (CLEANUP_FAILURE, "cannot remove intermediate"),
    ] {
        let mut fixture = Fixture::new();
        fixture.command.env(FIXTURE_FAILURE_ENV, stage);
        fixture.fails(error);
        if stage != MERGE_STAGE {
            assert!(!fixture.profile().exists());
        }
    }
    for (file, error) in [
        (JSON_FILE, "invalid llvm-cov JSON"),
        (REPORT_FILE, "function report"),
        (LCOV_FILE, "LCOV production sources differ"),
    ] {
        let mut fixture = Fixture::new();
        std::fs::write(fixture.root().join(file), "malformed").unwrap();
        fixture.fails(error);
        assert!(!fixture.profile().exists());
    }
}

#[test]
fn required_and_optional_environment_failures_are_isolated() {
    for name in [
        COVERAGE_DIRECTORY_ENV,
        EXECUTION_ROOT_ENV,
        RUNFILES_DIRECTORY_ENV,
        LLVM_COV_ENV,
        LLVM_PROFDATA_ENV,
        TEST_BINARY_ENV,
        TEST_WORKSPACE_ENV,
    ] {
        let mut fixture = Fixture::new();
        fixture.command.env_remove(name);
        fixture.fails(name);
    }
    for name in [
        INSTRUMENTED_TEST_BINARY_ENV,
        ADDITIONAL_COVERAGE_OBJECTS_ENV,
    ] {
        let mut fixture = Fixture::new();
        fixture.command.env(name, OsString::from_vec(vec![0xff]));
        fixture.fails(name);
    }
    let mut fixture = Fixture::new();
    fixture.command.env(
        ADDITIONAL_COVERAGE_OBJECTS_ENV,
        serde_json::to_string(&[FIXTURE_BINARY]).unwrap(),
    );
    fixture.fails("instrumented coverage object is duplicated");
}

#[test]
fn missing_tools_profiles_objects_and_unpublishable_reports_fail_closed() {
    for name in [LLVM_COV_ENV, LLVM_PROFDATA_ENV] {
        let mut fixture = Fixture::new();
        fixture.command.env(name, "unavailable");
        fixture.fails("tool is unavailable");
    }
    let mut fixture = Fixture::new();
    fixture.command.env(TEST_BINARY_ENV, "unavailable");
    fixture.fails("instrumented coverage object is unavailable");
    let mut fixture = Fixture::new();
    std::fs::remove_file(fixture.root().join(FIXTURE_COVERAGE).join(FIXTURE_RAW)).unwrap();
    fixture.fails("coverage run produced no raw profiles");
    let mut fixture = Fixture::new();
    fixture
        .command
        .env(COVERAGE_DIRECTORY_ENV, fixture.root().join("unavailable"));
    fixture.fails("cannot inspect");
    let mut fixture = Fixture::new();
    std::fs::create_dir(fixture.report()).unwrap();
    fixture.fails("cannot write");
}

#[test]
fn publication_cannot_claim_a_changed_or_uncreatable_completion_marker() {
    let mut fixture = Fixture::new();
    fixture.command.env(FIXTURE_FAILURE_ENV, MARKER_FAILURE);
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(FAILURE_EXIT_CODE));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot confirm coverage completion"));
    assert!(fixture.marker().exists());
    assert!(fixture.report().is_file());
    let mut fixture = Fixture::new();
    fixture
        .command
        .env(PREMATURE_EXIT_ENV, fixture.marker().join(FIXTURE_MARKER));
    let output = fixture.run();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cannot establish coverage completion marker")
    );
    assert!(!fixture.report().exists());
}

#[test]
fn saturated_execution_counts_cannot_publish_a_complete_report() {
    let saturated = i64::MAX as u64;
    for pointer in SATURATED_EXECUTION_PATHS {
        let mut fixture = Fixture::new();
        let path = fixture.root().join(JSON_FILE);
        let mut report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        *report.pointer_mut(pointer).unwrap() = saturated.into();
        std::fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();

        fixture.fails(&saturated.to_string());

        assert!(!fixture.profile().exists());
    }
}
