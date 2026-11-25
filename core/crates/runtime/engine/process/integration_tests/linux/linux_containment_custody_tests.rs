use super::*;
use crate::process::linux_containment::constants::{RECORD_SCHEMA_VERSION, session_leaf_name};
use std::fs::File;
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::PathBuf;

const TEST_BOUND_PID: u32 = 41;
const TEST_BOUND_START: u64 = 73;
const TEST_MISMATCH_OFFSET: u64 = 1;
const FCNTL_FAILURE: libc::c_int = -1;
const FORGED_ROOT: &str = "/tmp/../forged-root";

#[test]
fn non_cgroup_descriptors_fail_closed_and_close_the_pair() {
    let fixture = Fixture::new();
    let (root, leaf) = fixture.descriptors();
    let raw = [root.as_raw_fd(), leaf.as_raw_fd()];
    assert!(fixture.adopt(root, leaf, fixture.record.clone()).is_err());
    assert_closed(raw);
}

#[test]
fn inheritable_descriptor_rejection_closes_both_transferred_descriptors() {
    let fixture = Fixture::new();
    let (root, leaf) = fixture.descriptors();
    clear_close_on_exec(&root);
    let raw = [root.as_raw_fd(), leaf.as_raw_fd()];
    assert!(fixture.adopt(root, leaf, fixture.record.clone()).is_err());
    assert_closed(raw);

    let (root, leaf) = fixture.descriptors();
    clear_close_on_exec(&leaf);
    let raw = [root.as_raw_fd(), leaf.as_raw_fd()];
    assert!(fixture.adopt(root, leaf, fixture.record.clone()).is_err());
    assert_closed(raw);
}

#[test]
fn writable_descriptor_rejection_closes_both_transferred_descriptors() {
    let fixture = Fixture::new();
    let writable = tempfile::tempfile().unwrap();
    let (_, leaf) = fixture.descriptors();
    let root: OwnedFd = writable.into();
    let raw = [root.as_raw_fd(), leaf.as_raw_fd()];
    assert!(fixture.adopt(root, leaf, fixture.record.clone()).is_err());
    assert_closed(raw);
}

#[test]
fn runtime_bound_root_and_normalized_path_are_mandatory() {
    let fixture = Fixture::new();
    let mut wrong_session = fixture.record.clone();
    wrong_session.terminal_id = uuid::Uuid::new_v4();
    assert_rejected_and_closed(&fixture, wrong_session, fixture.root_identity());

    let mut wrong_epoch = fixture.record.clone();
    wrong_epoch.runtime_epoch = uuid::Uuid::new_v4();
    assert_rejected_and_closed(&fixture, wrong_epoch, fixture.root_identity());

    let mut wrong_root = fixture.record.clone();
    wrong_root.bound_start_time = Some(TEST_BOUND_START + TEST_MISMATCH_OFFSET);
    assert_rejected_and_closed(&fixture, wrong_root, fixture.root_identity());

    let wrong_expected_root = ProcessIdentity {
        pid: TEST_BOUND_PID,
        start_time: TEST_BOUND_START + TEST_MISMATCH_OFFSET,
    };
    assert_rejected_and_closed(&fixture, fixture.record.clone(), wrong_expected_root);

    let mut forged_path = fixture.record.clone();
    forged_path.delegated_root = PathBuf::from(FORGED_ROOT);
    assert_rejected_and_closed(&fixture, forged_path, fixture.root_identity());
}

fn assert_rejected_and_closed(
    fixture: &Fixture,
    record: LinuxContainmentRecord,
    root: ProcessIdentity,
) {
    let (root_fd, leaf_fd) = fixture.descriptors();
    let raw = [root_fd.as_raw_fd(), leaf_fd.as_raw_fd()];
    assert!(
        fixture
            .adopt_with_root(root_fd, leaf_fd, record, root)
            .is_err()
    );
    assert_closed(raw);
}

fn assert_closed(descriptors: [libc::c_int; super::super::transfer::DESCRIPTOR_COUNT]) {
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

fn clear_close_on_exec(descriptor: &OwnedFd) {
    let raw = descriptor.as_raw_fd();
    let flags = unsafe { libc::fcntl(raw, libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_ne!(
        unsafe { libc::fcntl(raw, libc::F_SETFD, flags & !libc::FD_CLOEXEC) },
        FCNTL_FAILURE
    );
}

struct Fixture {
    _directory: tempfile::TempDir,
    root: File,
    leaf: File,
    record: LinuxContainmentRecord,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let terminal_id = uuid::Uuid::new_v4();
        let runtime_epoch = uuid::Uuid::new_v4();
        let leaf_name = session_leaf_name(terminal_id, runtime_epoch);
        let leaf_path = directory.path().join(&leaf_name);
        std::fs::create_dir(&leaf_path).unwrap();
        let root = File::open(directory.path()).unwrap();
        let leaf = File::open(&leaf_path).unwrap();
        let record = LinuxContainmentRecord {
            schema_version: RECORD_SCHEMA_VERSION,
            terminal_id,
            runtime_epoch,
            boot_id: crate::process::boot_id(),
            delegated_root: directory.path().to_owned(),
            root_identity: super::super::descriptor::identity(&root).unwrap(),
            leaf_name,
            leaf_identity: super::super::descriptor::identity(&leaf).unwrap(),
            lease_token: uuid::Uuid::new_v4(),
            pty_device: None,
            pty_capability: None,
            bound_pid: Some(TEST_BOUND_PID),
            bound_start_time: Some(TEST_BOUND_START),
        };
        Self {
            _directory: directory,
            root,
            leaf,
            record,
        }
    }

    fn descriptors(&self) -> (OwnedFd, OwnedFd) {
        (
            self.root.try_clone().unwrap().into(),
            self.leaf.try_clone().unwrap().into(),
        )
    }

    fn root_identity(&self) -> ProcessIdentity {
        ProcessIdentity {
            pid: TEST_BOUND_PID,
            start_time: TEST_BOUND_START,
        }
    }

    fn adopt(
        &self,
        root: OwnedFd,
        leaf: OwnedFd,
        record: LinuxContainmentRecord,
    ) -> Result<LinuxContainmentCustody, LinuxContainmentError> {
        self.adopt_with_root(root, leaf, record, self.root_identity())
    }

    fn adopt_with_root(
        &self,
        root: OwnedFd,
        leaf: OwnedFd,
        record: LinuxContainmentRecord,
        bound_root: ProcessIdentity,
    ) -> Result<LinuxContainmentCustody, LinuxContainmentError> {
        LinuxContainmentCustody::from_transferred(
            root,
            leaf,
            record,
            self.record.terminal_id,
            self.record.runtime_epoch,
            bound_root,
        )
    }
}
