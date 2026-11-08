use super::docker_fixture::*;
use crate::container::engine::{Engine, Selector};
use yolo_release_gate_system::GateSnapshot;

#[test]
fn system_probe_uses_exact_arguments_and_releases_process_resource_uses() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let snapshot_root = snapshot.root_path();
    let mut docker = DockerFixture::new();
    let client_root = docker.root.path();
    let engine = docker.engine(&snapshot);
    assert_eq!(engine.identity().unwrap(), ENGINE_ID);
    assert_eq!(
        engine
            .find(Selector::Name(CONTAINER_NAME))
            .unwrap()
            .unwrap()
            .id,
        CONTAINER_ID
    );
    docker.write("filter", format!("id={CONTAINER_ID}").as_bytes());
    assert_eq!(
        engine.find(Selector::Id(CONTAINER_ID)).unwrap().unwrap().id,
        CONTAINER_ID
    );
    engine.remove(CONTAINER_ID).unwrap();
    assert_eq!(
        docker.calls(),
        [
            "info --format {{.ID}}".into(),
            format!("container ls --all --no-trunc --quiet --filter name=^/{CONTAINER_NAME}$"),
            format!("container inspect {CONTAINER_ID}"),
            format!("container ls --all --no-trunc --quiet --filter id={CONTAINER_ID}"),
            format!("container inspect {CONTAINER_ID}"),
            format!("container rm --force {CONTAINER_ID}"),
        ]
    );
    snapshot.close().unwrap();
    docker.root.cleanup().unwrap();
    assert!(!snapshot_root.exists());
    assert!(!client_root.exists());
}

#[test]
fn engine_identity_rejects_empty_non_graphic_and_non_utf8_daemon_output() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let docker = DockerFixture::new();
    let engine = docker.engine(&snapshot);
    for output in [
        b"".as_slice(),
        b" \n\t",
        b"two engines",
        b"engine\0id",
        b"\xff",
    ] {
        docker.write("identity", output);
        assert!(engine.identity().is_err(), "accepted {output:?}");
    }
    docker.write("identity", format!(" \n{ENGINE_ID}\n").as_bytes());
    assert_eq!(engine.identity().unwrap(), ENGINE_ID);
}

#[test]
fn list_rejects_ambiguous_or_invalid_ids_before_any_inspection() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let docker = DockerFixture::new();
    let engine = docker.engine(&snapshot);
    let multiple = format!("{CONTAINER_ID}\n{OTHER_ID}\n");
    for output in [b"\n".as_slice(), b"short", b"\xff", multiple.as_bytes()] {
        docker.write("list", output);
        let calls = docker.calls().len();
        assert!(engine.find(Selector::Name(CONTAINER_NAME)).is_err());
        assert_eq!(docker.calls().len(), calls + 1);
    }
    docker.write("list", b"");
    assert!(
        engine
            .find(Selector::Name(CONTAINER_NAME))
            .unwrap()
            .is_none()
    );
    docker.write("filter", format!("id={CONTAINER_ID}").as_bytes());
    docker.write("list", OTHER_ID.as_bytes());
    let calls = docker.calls().len();
    assert!(engine.find(Selector::Id(CONTAINER_ID)).is_err());
    assert_eq!(docker.calls().len(), calls + 1);
}

#[test]
fn inspection_requires_exact_single_selected_object_and_typed_shape() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let docker = DockerFixture::new();
    let engine = docker.engine(&snapshot);
    let one: serde_json::Value = serde_json::from_slice(&inspection(CONTAINER_ID)).unwrap();
    let duplicate = serde_json::to_vec(&serde_json::json!([one[0], one[0]])).unwrap();
    let wrong = inspection(OTHER_ID);
    for output in [
        b"[".as_slice(),
        b"{}",
        b"[]",
        b"[{}]",
        b"\xff",
        &duplicate,
        &wrong,
    ] {
        docker.write("inspect", output);
        assert!(
            engine.find(Selector::Name(CONTAINER_NAME)).is_err(),
            "accepted {output:?}"
        );
    }
    assert!(
        docker
            .calls()
            .iter()
            .all(|call| !call.starts_with("container rm"))
    );
}
