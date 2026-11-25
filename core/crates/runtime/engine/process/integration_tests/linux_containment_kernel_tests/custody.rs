use super::super::LinuxContainmentCustody;
use super::custody_fixture::{bind_pty, prepare_bound_lease};
use super::support::{
    SleepingChild, assert_frozen, freeze_members, leaf_identity, leaf_path, prepare, wait_empty,
};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};

const FCNTL_FAILURE: libc::c_int = -1;
const EXPECTED_MEMBER_COUNT: usize = 1;

#[test]
fn secondary_custody_rebuilds_from_exact_cloexec_descriptors_without_thawing() {
    let (terminal_id, runtime_epoch, mut lease) = prepare();
    let mut child = SleepingChild::spawn();
    let root = child.identity();
    lease.migrate_bootstrap(root).expect("migrate real member");
    let pty = bind_pty(&mut lease);
    let record = lease.record().clone();
    let path = leaf_path(&record);
    assert_eq!(freeze_members(&mut lease).len(), EXPECTED_MEMBER_COUNT);
    assert_frozen(&path);

    let exported = lease
        .export_secondary_custody()
        .expect("duplicate exact cgroup descriptors");
    assert_cloexec([exported.root_descriptor(), exported.leaf_descriptor()]);
    let recovery_identity = exported.recovery_identity_sha256();
    assert_eq!(exported.pty_device(), pty.device());
    assert_digest_serializes(recovery_identity);
    assert_ne!(
        recovery_identity,
        crate::sha256_digest::Sha256Digest::capture(&record).unwrap()
    );
    let (root_fd, leaf_fd) = exported.into_parts();
    let custody = LinuxContainmentCustody::from_transferred(
        root_fd,
        leaf_fd,
        record.clone(),
        terminal_id,
        runtime_epoch,
        root,
    )
    .expect("adopt exact cgroup custody");

    assert_eq!(custody.record(), &record);
    assert_eq!(custody.bound_root(), root);
    assert_eq!(
        custody.identities(),
        (record.root_identity, record.leaf_identity)
    );
    assert_eq!(custody.recovery_identity_sha256(), recovery_identity);
    assert_cloexec([custody.root_descriptor(), custody.leaf_descriptor()]);
    assert_frozen(&path);

    let (root_fd, leaf_fd) = custody.into_parts();
    assert_cloexec([root_fd.as_fd(), leaf_fd.as_fd()]);
    drop((root_fd, leaf_fd));
    child.terminate();
    wait_empty(&path);
    lease.remove_empty().expect("remove drained cgroup");
}

#[test]
fn foreign_leaf_cannot_forge_the_recorded_leaf_name_and_error_closes_both_fds() {
    let (terminal_id, runtime_epoch, mut lease) = prepare();
    let (foreign, mut foreign_child, _foreign_pty) = prepare_bound_lease();
    let mut child = SleepingChild::spawn();
    let root = child.identity();
    lease.migrate_bootstrap(root).expect("migrate real member");
    let _pty = bind_pty(&mut lease);
    let path = leaf_path(lease.record());
    assert_eq!(freeze_members(&mut lease).len(), EXPECTED_MEMBER_COUNT);
    let mut forged = lease.record().clone();
    forged.leaf_identity = leaf_identity(&foreign);

    let exported = lease.export_secondary_custody().unwrap();
    let foreign_exported = foreign.export_secondary_custody().unwrap();
    let (root_fd, local_leaf) = exported.into_parts();
    let (foreign_root, leaf_fd) = foreign_exported.into_parts();
    drop(local_leaf);
    drop(foreign_root);
    let raw = [root_fd.as_raw_fd(), leaf_fd.as_raw_fd()];
    assert!(
        LinuxContainmentCustody::from_transferred(
            root_fd,
            leaf_fd,
            forged,
            terminal_id,
            runtime_epoch,
            root,
        )
        .is_err()
    );
    assert_closed(raw);
    assert_frozen(&path);

    child.terminate();
    wait_empty(&path);
    foreign_child.terminate();
    wait_empty(&leaf_path(foreign.record()));
    lease.remove_empty().expect("remove local cgroup");
    foreign.remove_empty().expect("remove foreign cgroup");
}

#[test]
fn unbound_lease_cannot_export_secondary_custody() {
    let (_, _, lease) = prepare();
    assert!(lease.export_secondary_custody().is_err());
    lease.remove_empty().expect("remove unbound cgroup");
}

#[test]
fn bound_lease_without_pty_cannot_export_secondary_custody() {
    let (_, _, mut lease) = prepare();
    let mut child = SleepingChild::spawn();
    lease.migrate_bootstrap(child.identity()).unwrap();
    assert!(lease.export_secondary_custody().is_err());
    child.terminate();
    wait_empty(&leaf_path(lease.record()));
    lease.remove_empty().expect("remove PTY-unbound cgroup");
}

fn assert_cloexec(descriptors: [BorrowedFd<'_>; super::super::transfer::DESCRIPTOR_COUNT]) {
    for descriptor in descriptors {
        let flags = unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_GETFD) };
        assert_ne!(flags, FCNTL_FAILURE);
        assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
    }
}

pub(super) fn assert_closed(descriptors: [libc::c_int; super::super::transfer::DESCRIPTOR_COUNT]) {
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

fn assert_digest_serializes(digest: crate::sha256_digest::Sha256Digest) {
    let encoded = serde_json::to_vec(&digest).unwrap();
    assert_eq!(
        serde_json::from_slice::<crate::sha256_digest::Sha256Digest>(&encoded).unwrap(),
        digest
    );
}
