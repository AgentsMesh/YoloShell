use crate::{DockerClient, DockerClientTool};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use yolo_release_contracts::DOCKER_VERSION_ARGUMENT;
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_foundation::validation::SHA256_HEX_LENGTH;

const VERSION: &str = "Docker fixture 1.0";
const EXECUTABLE_MODE: u32 = 0o700;
const CLIENT_FIXTURE_NAME: &str = "docker";
const MOVED_EXTENSION: &str = "moved";
const REPLACEMENT_BYTES: &[u8] = b"changed";
const PRINTF_COMMAND: &str = "/usr/bin/printf";

#[test]
fn source_replacement_cannot_switch_the_pinned_client() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(CLIENT_FIXTURE_NAME);
    let bytes = format!("#!/bin/bash\n{PRINTF_COMMAND} '%s\\n' '{VERSION}'\n");
    fs::write(&path, bytes.as_bytes()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
    let client = DockerClient::capture(&path, &bytes_sha256(bytes.as_bytes()), VERSION).unwrap();
    client.ensure_unchanged().unwrap();
    fs::write(&path, REPLACEMENT_BYTES).unwrap();
    let output = std::process::Command::new(client.path())
        .arg(DOCKER_VERSION_ARGUMENT)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), VERSION);
    assert_ne!(client.path(), path);
    client.ensure_unchanged().unwrap();
}

#[test]
fn wrong_digest_or_version_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(CLIENT_FIXTURE_NAME);
    let bytes = format!("#!/bin/bash\n{PRINTF_COMMAND} 'actual\\n'\n");
    fs::write(&path, bytes.as_bytes()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
    assert!(DockerClient::capture(&path, &"0".repeat(SHA256_HEX_LENGTH), "actual").is_err());
    assert!(DockerClient::capture(&path, &bytes_sha256(bytes.as_bytes()), "expected").is_err());
}

#[test]
fn private_client_path_cannot_be_rebound() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(CLIENT_FIXTURE_NAME);
    let bytes = format!("#!/bin/bash\n{PRINTF_COMMAND} '%s\\n' '{VERSION}'\n");
    fs::write(&path, bytes.as_bytes()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
    let client = DockerClient::capture(&path, &bytes_sha256(bytes.as_bytes()), VERSION).unwrap();
    let moved = client.path().with_extension(MOVED_EXTENSION);
    assert_eq!(
        fs::rename(client.path(), moved).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    client.ensure_unchanged().unwrap();
}
