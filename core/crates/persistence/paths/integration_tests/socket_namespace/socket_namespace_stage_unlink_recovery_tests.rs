use super::*;
use crate::paths::private_file::{PrivateFileEntry, is_storage_artifact};
use crate::paths::private_file_lock::PrivateFileLock;
use std::ffi::CString;
use std::fs;
#[path = "../../test_support/integration/socket_stage_unlink_fixture.rs"]
mod fixture;
use fixture::Fixture;

#[test]
fn stage_unlink_recovery_reclaims_a_verified_interrupted_removal() {
    let fixture = Fixture::new();
    let namespace_path = fixture.directory.path().to_path_buf();
    assert!(fixture.isolation.path().is_file());
    run_scanner_pass(&fixture.namespace);
    assert!(!fixture.isolation.path().exists());
    drop(fixture);
    assert!(!namespace_path.exists());
}

#[test]
fn stage_unlink_recovery_does_not_remove_a_live_isolation_lock() {
    let fixture = Fixture::new();
    let lock = PrivateFileLock::try_acquire_existing_in(
        fixture.directory.try_clone().unwrap(),
        fixture.isolation.path(),
        SOCKET_STAGE_LEASE_MAX_BYTES,
    )
    .unwrap()
    .unwrap();
    run_scanner_pass(&fixture.namespace);
    assert!(fixture.isolation.path().exists());
    drop(lock);
    run_scanner_pass(&fixture.namespace);
    assert!(!fixture.isolation.path().exists());
}

#[test]
fn stage_unlink_recovery_preserves_unproven_data_and_mismatched_names() {
    for malformed in [true, false] {
        let fixture = Fixture::new();
        if malformed {
            fs::write(fixture.isolation.path(), INVALID_RECORD).unwrap();
        } else {
            let other = PrivateFileEntry::from_directory(
                fixture.directory.try_clone().unwrap(),
                &fixture.directory.path().join(OTHER_TARGET),
            )
            .unwrap()
            .unlink_isolation_entry()
            .unwrap();
            assert!(is_storage_artifact(other.path().file_name().unwrap()));
            fs::rename(fixture.isolation.path(), other.path()).unwrap();
            run_scanner_pass(&fixture.namespace);
            assert!(other.path().is_file());
            continue;
        }
        run_scanner_pass(&fixture.namespace);
        assert_eq!(fs::read(fixture.isolation.path()).unwrap(), INVALID_RECORD);
    }
}

#[test]
fn stage_unlink_recovery_preserves_a_replacement_canonical_lease() {
    let fixture = Fixture::new();
    let target = socket_stage::Target::new(
        fixture.directory.path(),
        &CString::new(HOST_SOCKET_FILE_NAME).unwrap(),
    )
    .unwrap();
    let canonical = target.lease().unwrap().1;
    fs::copy(fixture.isolation.path(), &canonical).unwrap();
    let lease = PrivateFileLock::try_acquire_existing_in(
        fixture.directory.try_clone().unwrap(),
        &canonical,
        SOCKET_STAGE_LEASE_MAX_BYTES,
    )
    .unwrap()
    .unwrap();
    let bytes = fs::read(&canonical).unwrap();
    run_scanner_pass(&fixture.namespace);
    assert!(!fixture.isolation.path().exists());
    assert_eq!(fs::read(&canonical).unwrap(), bytes);
    drop(lease);
}

const INVALID_RECORD: &[u8] = b"unproven stage lease";
const OTHER_TARGET: &str = "unrelated";
