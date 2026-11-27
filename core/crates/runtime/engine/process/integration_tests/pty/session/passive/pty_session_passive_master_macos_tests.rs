use super::*;
use crate::process::{MacosProcessCapabilityIssuer, MacosProcessCapabilityManagerReader};
use portable_pty::{PtySize, native_pty_system};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const TEST_CAPABILITY: &str = "passive-master-test-capability";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const MINIMUM_TEST_DESCRIPTOR: libc::c_int = libc::STDERR_FILENO + 1;
const FCNTL_FAILURE: libc::c_int = -1;
const VALID_DUPLICATE_DETAIL: &str = "valid passive PTY duplicate was rejected";
const OTHER_MASTER_ACCEPTED_DETAIL: &str = "another PTY master was accepted";
const REGULAR_FILE_ACCEPTED_DETAIL: &str = "regular file was accepted as a PTY master";
const READER_AS_MASTER_DETAIL: &str = "capability reader was accepted as a PTY master";
const MASTER_AS_READER_DETAIL: &str = "PTY master was accepted as a capability reader";

#[test]
fn controlled_duplicate_is_cloexec_and_bound_to_the_exact_pty() {
    let pair = open_pty();
    let proof = bound_proof(&pair);

    let passive = MacosPassivePtyMaster::duplicate_exact(&proof).expect(VALID_DUPLICATE_DETAIL);

    assert_cloexec(passive.descriptor().as_raw_fd());
    assert!(passive.matches(&proof));
    assert_eq!(passive.slave_path(), pair.master.tty_name().unwrap());
}

#[test]
fn transferred_master_is_made_cloexec_before_custody() {
    let pair = open_pty();
    let proof = bound_proof(&pair);
    let descriptor = duplicate(pair.master.as_raw_fd().unwrap(), libc::F_DUPFD);
    assert_not_cloexec(descriptor.as_raw_fd());

    let passive = MacosPassivePtyMaster::from_transferred_exact(descriptor, &proof)
        .expect(VALID_DUPLICATE_DETAIL);

    assert_cloexec(passive.descriptor().as_raw_fd());
    assert!(passive.matches(&proof));
}

#[test]
fn another_pty_master_is_rejected_and_closed() {
    let expected = open_pty();
    let other = open_pty();
    let proof = bound_proof(&expected);
    let descriptor = duplicate(other.master.as_raw_fd().unwrap(), libc::F_DUPFD);
    let raw = descriptor.as_raw_fd();

    MacosPassivePtyMaster::from_transferred_exact(descriptor, &proof)
        .err()
        .expect(OTHER_MASTER_ACCEPTED_DETAIL);

    assert_closed(raw);
}

#[test]
fn regular_file_is_rejected_and_closed() {
    let pair = open_pty();
    let proof = bound_proof(&pair);
    let descriptor: OwnedFd = tempfile::tempfile().unwrap().into();
    let raw = descriptor.as_raw_fd();

    MacosPassivePtyMaster::from_transferred_exact(descriptor, &proof)
        .err()
        .expect(REGULAR_FILE_ACCEPTED_DETAIL);

    assert_closed(raw);
}

#[test]
fn descriptor_roles_cannot_be_reversed() {
    let pair = open_pty();
    let proof = bound_proof(&pair);
    let (reader, _writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap();
    let identity = reader.identity();
    let capability_descriptor = reader.duplicate_for_secondary().unwrap();
    let capability_raw = capability_descriptor.as_raw_fd();

    MacosPassivePtyMaster::from_transferred_exact(capability_descriptor, &proof)
        .err()
        .expect(READER_AS_MASTER_DETAIL);
    assert_closed(capability_raw);

    let master_descriptor = duplicate(pair.master.as_raw_fd().unwrap(), libc::F_DUPFD);
    let master_raw = master_descriptor.as_raw_fd();
    MacosProcessCapabilityManagerReader::from_transferred_exact(master_descriptor, identity)
        .err()
        .expect(MASTER_AS_READER_DETAIL);
    assert_closed(master_raw);
}

fn bound_proof(pair: &portable_pty::PtyPair) -> PtySessionProof {
    PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap()
}

fn open_pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap()
}

fn duplicate(source: libc::c_int, operation: libc::c_int) -> OwnedFd {
    let raw = unsafe { libc::fcntl(source, operation, MINIMUM_TEST_DESCRIPTOR) };
    assert_ne!(raw, FCNTL_FAILURE);
    unsafe { OwnedFd::from_raw_fd(raw) }
}

fn assert_cloexec(descriptor: libc::c_int) {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}

fn assert_not_cloexec(descriptor: libc::c_int) {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_eq!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}

fn assert_closed(descriptor: libc::c_int) {
    assert_eq!(
        unsafe { libc::fcntl(descriptor, libc::F_GETFD) },
        FCNTL_FAILURE
    );
}
