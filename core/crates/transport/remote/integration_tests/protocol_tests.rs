use super::super::constants::{
    BRIDGE_COMMAND, HOME_ARGUMENT, MAX_REMOTE_PATH_BYTES, ROOT_PATH, STDIO_ARGUMENT, TEST_PATH_FILL,
};
use super::super::*;
use crate::model::HostStatus;
use std::path::PathBuf;

const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;

#[test]
fn endpoint_and_runtime_require_explicit_safe_values() {
    assert!(RemoteEndpoint::new("bad user", "host", 22).is_err());
    assert!(RemoteEndpoint::new("user", "host", 0).is_err());
    assert!(RemoteRuntimeSpec::new("yolo", "/state").is_err());
    assert!(RemoteRuntimeSpec::new("/", "/state").is_err());
    assert!(RemoteRuntimeSpec::new("/opt/../yolo", "/state").is_err());
    assert!(RemoteRuntimeSpec::new("/opt/yolo", "/").is_err());
    assert!(RemoteRuntimeSpec::new("/opt/yolo", "/var//yolo").is_err());
    assert!(RemoteRuntimeSpec::new("/opt/yolo\0host", "/state").is_err());
    let oversized = format!(
        "{ROOT_PATH}{}",
        TEST_PATH_FILL.repeat(MAX_REMOTE_PATH_BYTES)
    );
    assert!(RemoteRuntimeSpec::new(oversized, "/state").is_err());
    let runtime = RemoteRuntimeSpec::new("/opt/yolo/yolo", "/var/lib/yolo").unwrap();
    assert_eq!(runtime.executable(), std::path::Path::new("/opt/yolo/yolo"));
    assert_eq!(runtime.home(), std::path::Path::new("/var/lib/yolo"));
    assert_eq!(
        runtime.args(),
        [
            yolo_launch_contract::INTERNAL_COMMAND,
            yolo_launch_contract::ProcessRole::Host.name(),
            BRIDGE_COMMAND,
            STDIO_ARGUMENT,
            HOME_ARGUMENT,
            "/var/lib/yolo"
        ]
    );
}

#[test]
fn host_key_requires_two_openssh_fields() {
    assert!(HostKey::from_openssh("").is_err());
    let key = HostKey::from_openssh("ssh-ed25519 encoded comment").unwrap();
    assert_eq!(key.algorithm(), "ssh-ed25519");
    assert_eq!(key.encoded(), "encoded");
}

#[test]
fn runtime_validation_rejects_identity_version_home_and_capability_drift() {
    let runtime = RemoteRuntimeSpec::new("/opt/yolo/yolo", "/var/lib/yolo").unwrap();
    let mut status = compatible_status();
    super::super::protocol::validate_runtime(
        super::super::constants::RUNTIME_SERVER_NAME,
        crate::model::PROTOCOL_VERSION,
        &status,
        &runtime,
    )
    .unwrap();
    assert!(
        super::super::protocol::validate_runtime(
            "wrong-server",
            crate::model::PROTOCOL_VERSION,
            &status,
            &runtime,
        )
        .is_err()
    );
    status.home = PathBuf::from("/var/lib/other");
    assert!(
        super::super::protocol::validate_runtime(
            super::super::constants::RUNTIME_SERVER_NAME,
            crate::model::PROTOCOL_VERSION,
            &status,
            &runtime,
        )
        .is_err()
    );
    status = compatible_status();
    assert!(
        super::super::protocol::validate_runtime(
            super::super::constants::RUNTIME_SERVER_NAME,
            crate::model::PROTOCOL_VERSION + INCOMPATIBLE_PROTOCOL_VERSION_OFFSET,
            &status,
            &runtime,
        )
        .is_err()
    );
    status.protocol_version += INCOMPATIBLE_PROTOCOL_VERSION_OFFSET;
    assert!(
        super::super::protocol::validate_runtime(
            super::super::constants::RUNTIME_SERVER_NAME,
            crate::model::PROTOCOL_VERSION,
            &status,
            &runtime,
        )
        .is_err()
    );
    status = compatible_status();
    status.capabilities.pop();
    assert!(
        super::super::protocol::validate_runtime(
            super::super::constants::RUNTIME_SERVER_NAME,
            crate::model::PROTOCOL_VERSION,
            &status,
            &runtime,
        )
        .is_err()
    );
}

fn compatible_status() -> HostStatus {
    HostStatus {
        protocol_version: crate::model::PROTOCOL_VERSION,
        boot_id: "boot".into(),
        host_pid: 1,
        controller_generation: 1,
        home: PathBuf::from("/var/lib/yolo"),
        capabilities: super::super::constants::REQUIRED_RUNTIME_CAPABILITIES
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}
