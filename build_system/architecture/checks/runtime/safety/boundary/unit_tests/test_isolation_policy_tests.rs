use crate::test_isolation_contract::{
    COVERAGE_ENVIRONMENT, HOST_TESTS, ISOLATION_CREATE as CREATE, ISOLATION_CUSTODY as CUSTODY,
    ISOLATION_ENTRYPOINT as ENTRYPOINT, ISOLATION_FINALIZE as FINALIZE,
    ISOLATION_LAUNCHER as LAUNCHER, ISOLATION_RULE as RULE, ISOLATION_RUNNER as RUNNER,
    ISOLATION_WATCHDOG as WATCHDOG, LIFECYCLE_LOCK as LIFECYCLE, REMOTE_TESTS,
    RUNTIME_TESTS as BUILD,
};
use crate::test_isolation_contract::{FINALIZE_MARKERS, LIFECYCLE_MARKERS};
use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_direct_socket_owning_rust_tests() {
    let fixture = Fixture::new();
    fixture.write(
        BUILD,
        "rust_test(\n name = \"runtime_resilience_test\",\n)\n",
    );
    fixture.write(
        HOST_TESTS,
        "rust_test(\n name = \"host_unit_tests\",\n)\nrust_test(\n name = \"host_integration_tests\",\n)\n",
    );
    fixture.write(
        REMOTE_TESTS,
        "rust_test(\n name = \"remote_unit_tests\",\n)\nrust_test(\n name = \"remote_integration_tests\",\n)\n",
    );
    assert_eq!(messages(&fixture).len(), 5);
}

#[test]
fn accepts_all_socket_owning_targets_behind_the_isolation_rule() {
    let fixture = Fixture::new();
    fixture.write(
        BUILD,
        "isolated_rust_test(\n name = \"runtime_resilience_test\",\n)\n",
    );
    fixture.write(
        HOST_TESTS,
        "isolated_rust_test(\n name = \"host_unit_tests\",\n)\nisolated_rust_test(\n name = \"host_integration_tests\",\n)\n",
    );
    fixture.write(
        REMOTE_TESTS,
        "isolated_rust_test(\n name = \"remote_unit_tests\",\n)\nisolated_rust_test(\n name = \"remote_integration_tests\",\n)\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_incomplete_rule_and_destructive_rust_runner() {
    let fixture = Fixture::new();
    fixture.write(RULE, "isolated_rust_test()\n");
    fixture.write(
        ENTRYPOINT,
        "fn main() { std::fs::remove_dir_all(\"root\"); }\n",
    );
    assert_eq!(messages(&fixture).len(), 2);
}

#[test]
fn rejects_runner_that_drops_the_bazel_test_filter() {
    let fixture = Fixture::new();
    fixture.write(
        LAUNCHER,
        "impl LauncherClient { fn abort(&mut self) { Command::new(executable); SOCKET_ROOT_ENVIRONMENT; TEST_FILTER_ENVIRONMENT; .arg(test_filter); .stdin(Stdio::null()); force_unreaped_direct_child(&mut child); } }\n",
    );
    assert_eq!(messages(&fixture).len(), 1);
}

#[test]
fn rejects_incomplete_delegated_custody_layers() {
    let fixture = Fixture::new();
    fixture.write(RUNNER, "WatchdogClient::spawn();\n");
    fixture.write(WATCHDOG, "ControlCapability::generate();\n");
    fixture.write(CUSTODY, "IsolatedTestSocketRoot::create();\n");
    assert_eq!(messages(&fixture).len(), 3);
}

#[test]
fn rejects_incomplete_redundant_root_custody() {
    let runner = Fixture::new();
    runner.write(
        RUNNER,
        "WatchdogClient::spawn(); RootCustody::adopt(path); LauncherClient::spawn(); watchdog.track(process_root); TreeCustody::capture(); watchdog.finish(); root.disarm();\n",
    );
    assert_eq!(messages(&runner).len(), 1);

    let custody = Fixture::new();
    custody.write(
        CUSTODY,
        "IsolatedTestSocketRoot::create(); pub(super) fn disarm() {} CapturedProcessTree::capture_with_discovery(); watchdog_cleanup::settle(); self.finalize(); self.settle();\n",
    );
    assert_eq!(messages(&custody).len(), 1);
}

#[test]
fn rejects_incomplete_socket_root_capabilities() {
    let fixture = Fixture::new();
    fixture.write(CREATE, "pub fn create(path: &Path) {}\n");
    fixture.write(
        FINALIZE,
        "fn finalize() { std::fs::remove_dir_all(\"root\"); }\n",
    );
    fixture.write(LIFECYCLE, "fn lock() {}\n");
    assert_eq!(messages(&fixture).len(), 3);
}

#[test]
fn rejects_collector_that_can_only_see_the_outer_wrapper() {
    let fixture = Fixture::new();
    fixture.write(COVERAGE_ENVIRONMENT, "fn test_binary() {}\n");
    assert_eq!(messages(&fixture).len(), 1);
}

#[test]
fn accepts_current_directory_inode_custody_and_empty_only_finalization() {
    let fixture = Fixture::new();
    fixture.write(LIFECYCLE, &LIFECYCLE_MARKERS.join("\n"));
    fixture.write(FINALIZE, &FINALIZE_MARKERS.join("\n"));
    assert!(messages(&fixture).is_empty());
}

#[test]
fn every_inode_custody_and_finalization_fence_is_required_as_code() {
    for (path, markers) in [(LIFECYCLE, LIFECYCLE_MARKERS), (FINALIZE, FINALIZE_MARKERS)] {
        let original = markers.join("\n");
        for marker in markers {
            let fixture = Fixture::new();
            // Keeping the removed contract in a comment must not satisfy policy.
            let changed = original.replacen(marker, &format!("/* {marker} */"), 1);
            fixture.write(path, &changed);
            assert!(!messages(&fixture).is_empty(), "{path}: {marker}");
        }
    }
}

#[test]
fn shared_lock_wrong_inode_and_early_unlock_cannot_satisfy_custody() {
    for (path, original, before, after) in [
        (
            LIFECYCLE,
            LIFECYCLE_MARKERS,
            "Mode::Exclusive",
            "Mode::Shared",
        ),
        (
            LIFECYCLE,
            LIFECYCLE_MARKERS,
            "root.descriptor()",
            "foreign.descriptor()",
        ),
        (
            LIFECYCLE,
            LIFECYCLE_MARKERS,
            "self.root_identity",
            "other_identity",
        ),
        (
            FINALIZE,
            FINALIZE_MARKERS,
            "current == root.root_identity",
            "current == current",
        ),
        (
            FINALIZE,
            FINALIZE_MARKERS,
            "remove_retired(root)?; drop(lifecycle);",
            "drop(lifecycle); remove_retired(root)?;",
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(path, &original.join("\n").replace(before, after));
        assert!(!messages(&fixture).is_empty(), "{path}: {before}");
    }
}
