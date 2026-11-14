use super::*;
use crate::host::HostConfig;

#[path = "../../test_support/unit/socket/socket_namespace_certificate_test_support.rs"]
mod certificate_tamper;
#[path = "../../test_support/unit/socket/socket_namespace_registration_test_support.rs"]
mod registration_tamper;

#[path = "existing_presence_namespace.rs"]
mod namespace_security;

#[test]
fn absence_reservation_blocks_a_competing_host_without_creating_its_home() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join("not-started");
    let absence = Host::observe_absence(&home, None).unwrap().unwrap();
    assert!(!home.exists());
    assert!(Host::open(HostConfig::new(&home)).is_err());
    absence.verify().unwrap();
    drop(absence);
    let host = Host::open(HostConfig::new(&home)).unwrap();
    assert!(Host::observe_absence(&home, None).unwrap().is_none());
    drop(host);
    assert!(Host::observe_absence(&home, None).unwrap().is_some());
}

#[test]
fn a_created_or_replaced_home_invalidates_the_reserved_absence() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join("home");
    let absence = Host::observe_absence(&home, None).unwrap().unwrap();
    PrivateDirectory::ensure(&home).unwrap();
    assert!(absence.verify().is_err());
    drop(absence);
    let absence = Host::observe_absence(&home, None).unwrap().unwrap();
    std::fs::rename(&home, root.path().join("previous-home")).unwrap();
    PrivateDirectory::ensure(&home).unwrap();
    assert!(absence.verify().is_err());
}

#[test]
fn existing_or_new_socket_entries_never_become_absence() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join("home");
    let socket = root.path().join("existing.sock");
    std::fs::write(&socket, []).unwrap();
    assert!(
        Host::observe_absence(&home, Some(&socket))
            .unwrap()
            .is_none()
    );
    std::fs::remove_file(&socket).unwrap();
    let absence = Host::observe_absence(&home, Some(&socket))
        .unwrap()
        .unwrap();
    std::fs::write(&socket, []).unwrap();
    assert!(absence.verify().is_err());
}

#[test]
fn symlink_homes_are_errors_not_absence() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let target = root.path().join("target");
    PrivateDirectory::ensure(&target).unwrap();
    let home = root.path().join("link");
    std::os::unix::fs::symlink(&target, &home).unwrap();
    assert!(Host::observe_absence(&home, None).is_err());
}

#[test]
fn endpoint_reservation_prevents_absence_even_when_no_socket_was_published() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join("home");
    let socket = root.path().join("existing.sock");
    let path =
        crate::coordination::lock_path(CoordinationScope::HostEndpoint, &socket, None).unwrap();
    let lock = CoordinationLock::try_acquire(&path, HOST_INSTANCE_LOCK_MAX_BYTES)
        .unwrap()
        .unwrap();
    assert!(
        Host::observe_absence(&home, Some(&socket))
            .unwrap()
            .is_none()
    );
    drop(lock);
    assert!(
        Host::observe_absence(&home, Some(&socket))
            .unwrap()
            .is_some()
    );
}

#[test]
fn replaced_coordination_locks_invalidate_absence() {
    use std::os::unix::fs::OpenOptionsExt;

    for scope in [CoordinationScope::HostHome, CoordinationScope::HostEndpoint] {
        let root = tempfile::tempdir().unwrap();
        PrivateDirectory::ensure(root.path()).unwrap();
        let home = root.path().join("home");
        let socket = root.path().join("existing.sock");
        let absence = Host::observe_absence(&home, Some(&socket))
            .unwrap()
            .unwrap();
        let logical = match scope {
            CoordinationScope::HostHome => &home,
            _ => &socket,
        };
        let path = crate::coordination::lock_path(scope, logical, None).unwrap();
        std::fs::remove_file(&path).unwrap();
        let replacement = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(crate::paths::constants::PRIVATE_FILE_MODE)
            .open(&path)
            .unwrap();
        assert!(absence.verify().is_err());
        drop(replacement);
    }
}

#[test]
fn dangling_socket_symlinks_are_not_absence() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join("home");
    let socket = root.path().join("existing.sock");
    std::os::unix::fs::symlink(root.path().join("missing"), &socket).unwrap();
    assert!(
        Host::observe_absence(&home, Some(&socket))
            .unwrap()
            .is_none()
    );
}

#[test]
fn corrupted_namespace_registration_invalidates_previously_reserved_absence() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join("home");
    PrivateDirectory::ensure(&home).unwrap();
    let paths = Paths::new(&home);
    let namespace = paths.retain_socket_namespace().unwrap();
    let absence = Host::observe_absence(&home, None).unwrap().unwrap();
    let directory = paths.socket_dir().unwrap();
    let tamper = registration_tamper::CorruptRegistration::new(&home);
    assert!(absence.verify().is_err());
    drop(absence);
    drop(tamper);
    drop(namespace);
    drop(paths.retain_socket_namespace().unwrap());
    assert!(!directory.exists());
}
