use super::super::constants::{CGROUP_FREEZE_FILE, PIDS_MAX_FILE};
use super::super::{LinuxContainmentError, descriptor};
use super::record::DurableRecord;
use super::support::{
    SleepingChild, assert_frozen, freeze_members, leaf_identity, leaf_path, prepare, read_control,
    wait_empty,
};

const ALREADY_EXISTS_DETAIL: &str = "session cgroup already exists";
const RENAMED_LEAF_SUFFIX: &str = "-renamed";

#[test]
fn eexist_empty_leaf_fails_closed_without_modifying_old_leaf() {
    let (terminal_id, runtime_epoch, mut lease) = prepare();
    let path = leaf_path(lease.record());
    let record = DurableRecord::write(lease.record());
    let members = freeze_members(&mut lease);
    assert!(members.is_empty());
    let identity = leaf_identity(&lease);
    let freeze = read_control(&path, CGROUP_FREEZE_FILE);
    let limit = read_control(&path, PIDS_MAX_FILE);

    let error = super::support::broker()
        .prepare(terminal_id, runtime_epoch)
        .err()
        .expect("EEXIST must fail closed");
    assert!(matches!(
        error,
        LinuxContainmentError::Unavailable(ALREADY_EXISTS_DETAIL)
    ));
    assert_eq!(leaf_identity(&lease), identity);
    assert_eq!(read_control(&path, CGROUP_FREEZE_FILE), freeze);
    assert_eq!(read_control(&path, PIDS_MAX_FILE), limit);
    assert_frozen(&path);
    record.assert_unchanged();
    lease.remove_empty().expect("remove exact empty cgroup");
}

#[test]
fn eexist_nonempty_leaf_and_remove_busy_preserve_frozen_custody() {
    let (terminal_id, runtime_epoch, mut lease) = prepare();
    let mut child = SleepingChild::spawn();
    lease
        .migrate_bootstrap(child.identity())
        .expect("migrate real member");
    let record = DurableRecord::write(lease.record());
    let path = leaf_path(lease.record());
    let members = freeze_members(&mut lease);
    assert_eq!(members.len(), 1);
    let identity = leaf_identity(&lease);
    let freeze = read_control(&path, CGROUP_FREEZE_FILE);

    let error = super::support::broker()
        .prepare(terminal_id, runtime_epoch)
        .err()
        .expect("populated EEXIST must fail closed");
    assert!(matches!(
        error,
        LinuxContainmentError::Unavailable(ALREADY_EXISTS_DETAIL)
    ));
    assert_eq!(leaf_identity(&lease), identity);
    assert_eq!(read_control(&path, CGROUP_FREEZE_FILE), freeze);
    assert_frozen(&path);

    assert!(
        descriptor::remove_exact_leaf(&lease.root, &lease.leaf, lease.leaf_name()).is_err(),
        "a populated kernel cgroup must reject removal"
    );
    assert_eq!(leaf_identity(&lease), identity);
    assert_frozen(&path);

    let renamed = path.with_file_name(format!("{}{RENAMED_LEAF_SUFFIX}", lease.leaf_name()));
    assert!(
        std::fs::rename(&path, &renamed).is_err(),
        "cgroupfs must reject renaming a populated custody leaf"
    );
    assert!(!renamed.exists());
    assert_eq!(leaf_identity(&lease), identity);
    assert_frozen(&path);
    record.assert_unchanged();

    child.terminate();
    wait_empty(&path);
    lease.remove_empty().expect("remove exact drained cgroup");
}
