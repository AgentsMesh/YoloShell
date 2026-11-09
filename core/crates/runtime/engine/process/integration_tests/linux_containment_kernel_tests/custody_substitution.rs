use super::super::{LinuxContainmentCustody, LinuxContainmentRecord};
use super::custody::assert_closed;
use super::custody_fixture::{bind_pty, prepare_bound_lease};
use super::support::{SleepingChild, leaf_path, prepare, wait_empty};
use crate::process::ProcessIdentity;
use std::os::fd::{AsRawFd, OwnedFd};
use uuid::Uuid;

const IDENTITY_MISMATCH_OFFSET: u64 = 1;

#[test]
fn root_and_leaf_inode_substitution_are_rejected() {
    let (terminal_id, runtime_epoch, mut lease) = prepare();
    let (foreign, mut foreign_child, _foreign_pty) = prepare_bound_lease();
    let mut child = SleepingChild::spawn();
    let root = child.identity();
    lease.migrate_bootstrap(root).expect("migrate real member");
    let _pty = bind_pty(&mut lease);
    let record = lease.record().clone();

    let local = lease.export_secondary_custody().unwrap();
    let alternate = foreign.export_secondary_custody().unwrap();
    let (local_root, local_leaf) = local.into_parts();
    let (shared_root, foreign_leaf) = alternate.into_parts();
    drop(shared_root);
    assert_ne!(foreign.record().leaf_identity, record.root_identity);
    let foreign_root = foreign_leaf
        .try_clone()
        .expect("duplicate genuinely foreign root inode");
    let mount_substitution = lease.export_secondary_custody().unwrap();
    let (mount_root, mount_leaf) = mount_substitution.into_parts();
    let mut forged_mount = record.clone();
    forged_mount.root_identity.mount_id += IDENTITY_MISMATCH_OFFSET;
    assert_rejected(
        mount_root,
        mount_leaf,
        forged_mount,
        terminal_id,
        runtime_epoch,
        root,
    );
    assert_rejected(
        foreign_root,
        local_leaf,
        record.clone(),
        terminal_id,
        runtime_epoch,
        root,
    );
    assert_rejected(
        local_root,
        foreign_leaf,
        record,
        terminal_id,
        runtime_epoch,
        root,
    );

    child.terminate();
    wait_empty(&leaf_path(lease.record()));
    foreign_child.terminate();
    wait_empty(&leaf_path(foreign.record()));
    lease.remove_empty().expect("remove local cgroup");
    foreign.remove_empty().expect("remove foreign cgroup");
}

fn assert_rejected(
    root_fd: OwnedFd,
    leaf_fd: OwnedFd,
    record: LinuxContainmentRecord,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    root: ProcessIdentity,
) {
    let raw = [root_fd.as_raw_fd(), leaf_fd.as_raw_fd()];
    assert!(
        LinuxContainmentCustody::from_transferred(
            root_fd,
            leaf_fd,
            record,
            terminal_id,
            runtime_epoch,
            root,
        )
        .is_err()
    );
    assert_closed(raw);
}
