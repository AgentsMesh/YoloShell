use super::{test_support::*, *};
use std::{ffi::CString, fs};

const UNSAFE_STAGE_MODE: u32 = 0o700;

#[test]
fn certified_namespace_scanner_reclaims_crashed_managed_stages() {
    for kind in [socket_stage::Kind::Bind, socket_stage::Kind::Probe] {
        let fixture = RegisteredFixture::new();
        let owner = fixture.paths.retain_socket_namespace().unwrap();
        let directory = owner.directory().unwrap();
        let stage = create_crash_stage(directory.path(), kind, PRIVATE_FILE_MODE);
        run_scanner_pass(&owner);
        assert!(!stage.exists());
        owner.verify().unwrap();
        assert!(directory.path().exists());
        assert_eq!(owner.release().unwrap(), NamespaceRelease::Removed);
        assert!(!directory.path().exists());
    }
}

#[test]
fn live_namespace_scans_keep_stage_lifecycle_until_final_release() {
    let fixture = RegisteredFixture::new();
    let owner = fixture.paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    let target = socket_stage::Target::new(
        directory.path(),
        &CString::new(HOST_SOCKET_FILE_NAME).unwrap(),
    )
    .unwrap();
    let lease = socket_stage::Lease::acquire_immediate_for_test(&directory, &target).unwrap();
    let lifecycle = target.lifecycle_path();
    let identity = inode(&lifecycle);
    let lease_path = target.lease().unwrap().1;
    drop(lease);
    run_scanner_pass(&owner);
    assert!(!lease_path.exists());
    assert_eq!(inode(&lifecycle), identity);
    assert_eq!(owner.release().unwrap(), NamespaceRelease::Removed);
    assert!(!directory.path().exists());
}

#[test]
fn unproven_stage_is_preserved_and_does_not_authorize_directory_removal() {
    let fixture = RegisteredFixture::new();
    let owner = fixture.paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    let stage = create_crash_stage(
        directory.path(),
        socket_stage::Kind::Bind,
        UNSAFE_STAGE_MODE,
    );
    let identity = inode(&stage);
    run_scanner_pass(&owner);
    assert_eq!(inode(&stage), identity);
    assert_eq!(owner.release().unwrap(), NamespaceRelease::Preserved);
    assert_eq!(inode(&stage), identity);
}

#[test]
fn a_scanner_never_reclaims_an_uncertified_directory_or_unknown_entry() {
    let fixture = RegisteredFixture::new();
    let location =
        SocketNamespaceLocation::locate(fixture.paths.home(), fixture.paths.socket_root_override())
            .unwrap();
    let directory = PrivateDirectory::ensure(&location.directory).unwrap();
    let stage = create_crash_stage(
        directory.path(),
        socket_stage::Kind::Bind,
        PRIVATE_FILE_MODE,
    );
    let root = PrivateDirectory::open(&location.root).unwrap();
    let mut scanner = SocketNamespaceScanner::from_root(&root, true).unwrap();
    finish_scanner_pass(&mut scanner);
    assert!(fs::symlink_metadata(&stage).is_ok());
    directory.verify_path().unwrap();
}
