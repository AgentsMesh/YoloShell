use super::super::{cleanup_io, probe_target};
use super::*;
use crate::paths::constants::{
    EXPECTED_FILE_LINK_COUNT, MAX_PRIVATE_DIRECTORY_ENTRIES, SOCKET_FILE_EXTENSION,
};
use uuid::Uuid;

const OVERFLOW_ENTRY_COUNT: usize = 1;
const FIRST_OVERFLOW_ENTRY: usize = 0;
const OVERFLOW_ENTRY_PREFIX: &str = "bounded-scan-entry";
const MISSING_ENTRY_NAME: &str = "missing-stage-entry";
const RENAME_SOURCE_NAME: &str = "rename-stage-source";
const RENAME_TARGET_NAME: &str = "rename-stage-target";
const UNSAFE_ENDPOINT_BYTES: &[u8] = b"not a socket";
const UNKNOWN_DEVICE: libc::dev_t = 0;
const UNKNOWN_INODE: libc::ino_t = 0;

fn unknown_identity() -> cleanup_io::Identity {
    cleanup_io::Identity {
        device: UNKNOWN_DEVICE,
        inode: UNKNOWN_INODE,
    }
}

#[test]
fn explicit_and_scanned_probe_evidence_rejects_an_unsafe_endpoint() {
    let (_temporary, directory, target) = super::fixture();
    let endpoint_name = CString::new(HOST_SOCKET_FILE_NAME).unwrap();
    fs::write(
        directory.path().join(HOST_SOCKET_FILE_NAME),
        UNSAFE_ENDPOINT_BYTES,
    )
    .unwrap();

    assert!(
        !probe_target::has_links(
            &directory,
            &target,
            Some(&endpoint_name),
            unknown_identity(),
            EXPECTED_FILE_LINK_COUNT,
        )
        .unwrap()
    );
    assert!(
        !probe_target::has_links(
            &directory,
            &target,
            None,
            unknown_identity(),
            EXPECTED_FILE_LINK_COUNT,
        )
        .unwrap()
    );
}

#[test]
fn scanned_probe_evidence_is_bounded_after_unrelated_endpoints() {
    let (_temporary, directory, target) = super::fixture();
    let unrelated = format!("{}{}", Uuid::nil(), SOCKET_FILE_EXTENSION);
    fs::write(directory.path().join(unrelated), UNSAFE_ENDPOINT_BYTES).unwrap();
    for index in FIRST_OVERFLOW_ENTRY..=MAX_PRIVATE_DIRECTORY_ENTRIES + OVERFLOW_ENTRY_COUNT {
        let name = format!("{OVERFLOW_ENTRY_PREFIX}-{index}");
        fs::write(directory.path().join(name), UNSAFE_ENDPOINT_BYTES).unwrap();
    }

    assert!(
        !probe_target::has_links(
            &directory,
            &target,
            None,
            unknown_identity(),
            EXPECTED_FILE_LINK_COUNT,
        )
        .unwrap()
    );
}

#[test]
fn cleanup_io_distinguishes_missing_sources_from_exclusive_collisions() {
    let (_temporary, directory, _target) = super::fixture();
    let missing = CString::new(MISSING_ENTRY_NAME).unwrap();
    assert!(cleanup_io::unlink(&directory, &missing).is_err());

    let source = CString::new(RENAME_SOURCE_NAME).unwrap();
    let target = CString::new(RENAME_TARGET_NAME).unwrap();
    assert!(cleanup_io::rename_exclusive(&directory, &source, &target).is_err());
    fs::write(
        directory.path().join(RENAME_SOURCE_NAME),
        UNSAFE_ENDPOINT_BYTES,
    )
    .unwrap();
    fs::write(
        directory.path().join(RENAME_TARGET_NAME),
        UNSAFE_ENDPOINT_BYTES,
    )
    .unwrap();
    assert!(!cleanup_io::rename_exclusive(&directory, &source, &target).unwrap());
}
