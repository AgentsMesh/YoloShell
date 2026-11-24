use super::devpts_fixture::PtyPair;
use super::secondary_custody_fixture::SecondaryCustodyFixture;
use crate::process::LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT;
use crate::sha256_digest::Sha256Digest;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

const PTY_MASTER_INDEX: usize = 0;
const CGROUP_ROOT_INDEX: usize = 1;
const CGROUP_LEAF_INDEX: usize = 2;
const FIRST_PRIVATE_DESCRIPTOR: libc::c_int = libc::STDERR_FILENO + 1;
const FCNTL_FAILURE: libc::c_int = -1;
const FOREIGN_DIGEST_BYTE: u8 = 0xa7;
const PTY_MULTIPLEXER_PATH: &str = "/dev/ptmx";
const REVERSED_ROLE_DETAIL: &str = "containment custody root descriptor identity mismatch";
const FOREIGN_PTY_DETAIL: &str = "transferred PTY master belongs to another slave device";
const DIGEST_MISMATCH_DETAIL: &str =
    "transferred Linux custody descriptors have another recovery identity";
const INHERITABLE_DETAIL: &str = "passive PTY master descriptor is inheritable";
const ACCESS_MODE_DETAIL: &str = "passive PTY master has an invalid access mode";
const FAILURE_WAS_ACCEPTED: &str = "invalid three-descriptor custody was accepted";

#[test]
fn three_descriptor_custody_imports_as_one_exact_authority() {
    let fixture = SecondaryCustodyFixture::new();
    let (descriptors, digest) = fixture.descriptors();
    let custody = fixture.adopt(descriptors, digest).unwrap();
    assert_eq!(custody.recovery_identity_sha256(), digest);
    drop(custody);
    fixture.finish();
}

#[test]
fn active_secondary_custody_settlement_kills_the_exact_cgroup() {
    let fixture = SecondaryCustodyFixture::new();
    let (descriptors, digest) = fixture.descriptors();
    let custody = fixture.adopt(descriptors, digest).unwrap();
    assert!(!custody.closure_settled().unwrap());

    assert!(!custody.request_active_settlement().unwrap());
    let deadline = std::time::Instant::now()
        .checked_add(crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT)
        .unwrap();
    while !custody.closure_settled().unwrap() && std::time::Instant::now() < deadline {
        std::thread::sleep(super::super::constants::FREEZE_POLL_INTERVAL);
    }
    assert!(custody.closure_settled().unwrap());
    drop(custody);
    fixture.finish();
}

#[test]
fn reversed_cgroup_roles_fail_closed_for_all_descriptors() {
    let fixture = SecondaryCustodyFixture::new();
    let (mut descriptors, digest) = fixture.descriptors();
    descriptors.swap(CGROUP_ROOT_INDEX, CGROUP_LEAF_INDEX);
    assert_rejected_and_closed(&fixture, descriptors, digest, REVERSED_ROLE_DETAIL);
    fixture.finish();
}

#[test]
fn foreign_pty_master_fails_closed_for_all_descriptors() {
    let fixture = SecondaryCustodyFixture::new();
    let foreign = PtyPair::open();
    let (mut descriptors, digest) = fixture.descriptors();
    descriptors[PTY_MASTER_INDEX] = duplicate_cloexec(foreign.master.as_raw_fd());
    assert_rejected_and_closed(&fixture, descriptors, digest, FOREIGN_PTY_DETAIL);
    fixture.finish();
}

#[test]
fn composite_digest_mismatch_fails_closed_for_all_descriptors() {
    let fixture = SecondaryCustodyFixture::new();
    let (descriptors, _) = fixture.descriptors();
    let foreign_digest = Sha256Digest::repeated(FOREIGN_DIGEST_BYTE);
    assert_rejected_and_closed(
        &fixture,
        descriptors,
        foreign_digest,
        DIGEST_MISMATCH_DETAIL,
    );
    fixture.finish();
}

#[test]
fn inheritable_pty_master_fails_closed_for_all_descriptors() {
    let fixture = SecondaryCustodyFixture::new();
    let (descriptors, digest) = fixture.descriptors();
    clear_close_on_exec(descriptors[PTY_MASTER_INDEX].as_raw_fd());
    assert_rejected_and_closed(&fixture, descriptors, digest, INHERITABLE_DETAIL);
    fixture.finish();
}

#[test]
fn non_read_write_master_fails_closed_for_all_descriptors() {
    let fixture = SecondaryCustodyFixture::new();
    let (mut descriptors, digest) = fixture.descriptors();
    let read_only = File::open(PTY_MULTIPLEXER_PATH).unwrap();
    descriptors[PTY_MASTER_INDEX] = read_only.into();
    assert_rejected_and_closed(&fixture, descriptors, digest, ACCESS_MODE_DETAIL);
    fixture.finish();
}

fn assert_rejected_and_closed(
    fixture: &SecondaryCustodyFixture,
    descriptors: [OwnedFd; LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT],
    digest: Sha256Digest,
    expected_detail: &str,
) {
    let raw = raw_descriptors(&descriptors);
    let error = fixture
        .adopt(descriptors, digest)
        .err()
        .expect(FAILURE_WAS_ACCEPTED);
    assert_closed(raw);
    assert!(error.to_string().contains(expected_detail), "{error}");
}

fn raw_descriptors(
    descriptors: &[OwnedFd; LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT],
) -> [RawFd; LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT] {
    descriptors
        .each_ref()
        .map(|descriptor| descriptor.as_raw_fd())
}

fn assert_closed(descriptors: [RawFd; LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT]) {
    for descriptor in descriptors {
        assert_eq!(
            unsafe { libc::fcntl(descriptor, libc::F_GETFD) },
            FCNTL_FAILURE
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EBADF)
        );
    }
}

fn clear_close_on_exec(descriptor: RawFd) {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_eq!(
        unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags & !libc::FD_CLOEXEC) },
        libc::c_int::default()
    );
}

fn duplicate_cloexec(descriptor: RawFd) -> OwnedFd {
    let duplicate =
        unsafe { libc::fcntl(descriptor, libc::F_DUPFD_CLOEXEC, FIRST_PRIVATE_DESCRIPTOR) };
    assert_ne!(duplicate, FCNTL_FAILURE);
    unsafe { OwnedFd::from_raw_fd(duplicate) }
}
