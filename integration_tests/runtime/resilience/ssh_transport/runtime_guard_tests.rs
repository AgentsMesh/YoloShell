use super::*;

const TEST_CONTROLLER_GENERATION: u64 = 7;
const CHANGED_CONTROLLER_GENERATION: u64 = TEST_CONTROLLER_GENERATION + 1;
const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;
const CHANGED_PROTOCOL_VERSION: u16 =
    yolo_runtime::model::PROTOCOL_VERSION.saturating_add(INCOMPATIBLE_PROTOCOL_VERSION_OFFSET);
const CHANGED_PID_OFFSET: u32 = 1;
const CHANGED_BOOT_ID: &str = "changed-test-boot";
const TEST_BOOT_ID: &str = "detached-remote-test-boot";
const UNKNOWN_BOOT_ID: &str = "unknown:detached-remote-test";

#[derive(Clone, Copy)]
enum StatusMutation {
    Protocol,
    Home,
    Pid,
    Boot,
    Generation,
}

#[test]
fn status_binding_rejects_every_changed_host_authority_field() {
    for mutation in [
        StatusMutation::Protocol,
        StatusMutation::Home,
        StatusMutation::Pid,
        StatusMutation::Boot,
        StatusMutation::Generation,
    ] {
        assert_status_change_rejected(mutation);
    }
}

#[test]
fn cleanup_retry_is_fail_closed_for_identity_observation() {
    assert!(!cleanup_requires_retry(
        ProcessIdentityObservation::Gone,
        false
    ));
    assert!(cleanup_requires_retry(
        ProcessIdentityObservation::Gone,
        true
    ));
    assert!(cleanup_requires_retry(
        ProcessIdentityObservation::Matching,
        false
    ));
    assert!(cleanup_requires_retry(
        ProcessIdentityObservation::Unknown,
        false
    ));
}

#[test]
fn status_binding_rejects_untrusted_boot_authority() {
    let home = tempfile::tempdir().unwrap();
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let mut guard = DetachedRuntimeGuard::from_peer(home.path(), identity);
    guard.disarm_for_test();
    let mut status = status(home.path(), identity.pid);
    status.boot_id = UNKNOWN_BOOT_ID.into();
    assert!(guard.verify_status(&status).is_err());
}

#[test]
fn failed_boot_change_does_not_replace_bound_cleanup_authority() {
    let home = tempfile::tempdir().unwrap();
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let mut guard = DetachedRuntimeGuard::from_peer(home.path(), identity);
    guard.disarm_for_test();
    let original = status(home.path(), identity.pid);
    guard.verify_status(&original).unwrap();
    let mut changed = original.clone();
    changed.boot_id = CHANGED_BOOT_ID.into();

    assert!(guard.verify_status(&changed).is_err());
    assert_eq!(guard.boot_id.as_deref(), Some(TEST_BOOT_ID));
}

fn assert_status_change_rejected(mutation: StatusMutation) {
    let home = tempfile::tempdir().unwrap();
    let other_home = tempfile::tempdir().unwrap();
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let mut guard = DetachedRuntimeGuard::from_peer(home.path(), identity);
    guard.disarm_for_test();
    let mut status = status(home.path(), identity.pid);
    guard.verify_status(&status).unwrap();
    match mutation {
        StatusMutation::Protocol => status.protocol_version = CHANGED_PROTOCOL_VERSION,
        StatusMutation::Home => status.home = other_home.path().to_path_buf(),
        StatusMutation::Pid => status.host_pid += CHANGED_PID_OFFSET,
        StatusMutation::Boot => status.boot_id = CHANGED_BOOT_ID.into(),
        StatusMutation::Generation => status.controller_generation = CHANGED_CONTROLLER_GENERATION,
    }
    assert!(guard.verify_status(&status).is_err());
}

fn status(home: &Path, host_pid: u32) -> HostStatus {
    HostStatus {
        protocol_version: yolo_runtime::model::PROTOCOL_VERSION,
        boot_id: TEST_BOOT_ID.into(),
        host_pid,
        controller_generation: TEST_CONTROLLER_GENERATION,
        home: Paths::new(home).home().to_path_buf(),
        capabilities: Vec::new(),
    }
}
