use super::{DockerClient, DockerClientTool};
use crate::integration_tests::cleanup;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use yolo_release_contracts::PRIVATE_EXECUTABLE_MODE;
use yolo_release_foundation::digest::bytes_sha256;

const SOURCE_NAME: &str = "docker-fixture";
const MOVED_NAME: &str = "retained-original";
const PROBE_NAME: &str = "private-path";
const VERSION: &str = "Docker cleanup fixture";

#[test]
fn docker_client_drop_preserves_replacement_after_integrity_failure() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join(SOURCE_NAME);
    let script = format!("#!/bin/bash\nprintf '%s\\n' '{VERSION}'\n");
    executable(&source, &script);
    let client = DockerClient::capture(&source, &bytes_sha256(script.as_bytes()), VERSION).unwrap();
    let private = client.path().parent().unwrap().to_owned();
    cleanup::rebind(&private, &fixture.path().join(MOVED_NAME));
    assert!(client.ensure_unchanged().is_err());
    drop(client);
    cleanup::assert_preserved_and_remove(&private);
}

#[test]
fn failed_docker_version_capture_cannot_delete_replaced_private_root() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join(SOURCE_NAME);
    let probe = fixture.path().join(PROBE_NAME);
    let script = cleanup::failed_capture_script(&probe, &fixture.path().join(MOVED_NAME));
    executable(&source, &script);
    assert!(DockerClient::capture(&source, &bytes_sha256(script.as_bytes()), VERSION).is_err());
    let private = std::path::PathBuf::from(fs::read_to_string(probe).unwrap());
    cleanup::assert_preserved_and_remove(&private);
}

fn executable(path: &std::path::Path, script: &str) {
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_EXECUTABLE_MODE)).unwrap();
}
