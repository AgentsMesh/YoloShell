use super::docker_fixture::*;
use crate::container::engine::{Engine, Selector};
use yolo_release_contracts::{CONTAINER_ALL_ARGUMENT, CONTAINER_CHANGED, CONTAINER_CONTROL_FAILED};
use yolo_release_gate_system::GateSnapshot;
use yolo_release_tool_custody::DockerClientTool;

const BEFORE_EXECUTION_CHECK: usize = 1;
const AFTER_EXECUTION_CHECK: usize = 2;

#[test]
fn invalid_id_never_dispatches_inspection_or_destructive_command() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let docker = DockerFixture::new();
    let engine = docker.engine(&snapshot);
    for id in [
        "",
        "short",
        CONTAINER_ALL_ARGUMENT,
        &CONTAINER_ID.to_uppercase(),
    ] {
        let error = engine.find(Selector::Id(id)).unwrap_err();
        assert!(format!("{error:#}").contains(CONTAINER_CHANGED));
        assert!(engine.remove(id).is_err());
    }
    assert!(docker.calls().is_empty());
    assert_eq!(docker.checks.get(), 0);
}

#[test]
fn daemon_exit_failure_preserves_diagnostic_and_never_becomes_absence() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let mut docker = DockerFixture::new();
    docker.write("reject", DAEMON_FAILURE.as_bytes());
    let engine = docker.engine(&snapshot);
    for result in [
        engine.identity().map(|_| ()),
        engine.find(Selector::Name(CONTAINER_NAME)).map(|_| ()),
        engine.remove(CONTAINER_ID),
    ] {
        let error = format!("{:#}", result.unwrap_err());
        assert!(error.contains(CONTAINER_CONTROL_FAILED), "{error}");
        assert!(error.contains(DAEMON_FAILURE), "{error}");
    }
    snapshot.close().unwrap();
    docker.root.cleanup().unwrap();
}

#[test]
fn client_validation_before_and_after_execution_both_fail_closed() {
    for failed_check in [BEFORE_EXECUTION_CHECK, AFTER_EXECUTION_CHECK] {
        let snapshot = GateSnapshot::fixture().unwrap();
        let mut docker = DockerFixture::new();
        docker.fail_check.set(Some(failed_check));
        let error = docker.engine(&snapshot).identity().unwrap_err();
        assert!(format!("{error:#}").contains(CLIENT_CHANGED));
        assert_eq!(docker.calls().len(), failed_check - BEFORE_EXECUTION_CHECK);
        assert_eq!(docker.checks.get(), failed_check);
        snapshot.close().unwrap();
        docker.root.cleanup().unwrap();
    }
}

#[test]
fn missing_client_returns_spawn_error_without_poisoning_known_unsent_resources() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let mut docker = DockerFixture::new();
    std::fs::remove_file(docker.path()).unwrap();
    assert!(docker.engine(&snapshot).identity().is_err());
    assert!(docker.calls().is_empty());
    snapshot.close().unwrap();
    docker.root.cleanup().unwrap();
}
