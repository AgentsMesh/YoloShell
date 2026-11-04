use super::super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const TARGET_NAME: &str = "recovery-instance.bin";
const SAVED_NAME: &str = "saved-recovery-instance";
const ISOLATED_BYTES: &[u8] = b"isolated-instance";
const CHANGED_BYTES: &[u8] = b"changed-recovery-instance";
const OCCUPANT_BYTES: &[u8] = b"isolated-occupant";

#[derive(Clone, Copy, Debug)]
enum Operation {
    Restore,
    Recycle,
}

#[derive(Clone, Copy, Debug)]
enum Mutation {
    None,
    InitialReplacement,
    OccupiedDestination,
    Rewrite,
    Displace,
}

#[test]
fn shared_recovery_instances_cover_success_rollback_and_preservation() {
    for operation in [Operation::Restore, Operation::Recycle] {
        for mutation in [
            Mutation::None,
            Mutation::InitialReplacement,
            Mutation::OccupiedDestination,
            Mutation::Rewrite,
            Mutation::Displace,
        ] {
            run_case(operation, mutation);
        }
    }
}

fn run_case(operation: Operation, mutation: Mutation) {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let isolated_name = private_file_names::retained_isolation_name(entry.file_name()).unwrap();
    let retired_name = private_file_names::retired_name(entry.file_name()).unwrap();
    let isolated = named_path(&target, &isolated_name);
    let retired = named_path(&target, &retired_name);
    write_private(&isolated, ISOLATED_BYTES);
    let opened = entry.open_isolated_retained(u64::MAX).unwrap().unwrap();
    let expected = opened.identity;
    drop(opened);
    let destination = match operation {
        Operation::Restore => target.clone(),
        Operation::Recycle => retired,
    };
    match mutation {
        Mutation::InitialReplacement => {
            fs::remove_file(&isolated).unwrap();
            write_private(&isolated, CHANGED_BYTES);
        }
        Mutation::OccupiedDestination => write_private(&destination, OCCUPANT_BYTES),
        Mutation::None | Mutation::Rewrite | Mutation::Displace => {}
    }
    let saved = root.path().join(SAVED_NAME);
    let after_move = || match mutation {
        Mutation::None | Mutation::InitialReplacement | Mutation::OccupiedDestination => {}
        Mutation::Rewrite => fs::write(&destination, CHANGED_BYTES).unwrap(),
        Mutation::Displace => {
            fs::rename(&destination, &saved).unwrap();
            write_private(&isolated, OCCUPANT_BYTES);
        }
    };
    let result = match operation {
        Operation::Restore => {
            entry.restore_isolated_retained_with_hook(expected, u64::MAX, after_move)
        }
        Operation::Recycle => {
            entry.recycle_isolated_retained_with_hook(expected, u64::MAX, after_move)
        }
    };
    assert_eq!(result.is_ok(), matches!(mutation, Mutation::None));
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn named_path(target: &Path, name: &std::ffi::CStr) -> PathBuf {
    target.with_file_name(std::ffi::OsStr::from_bytes(name.to_bytes()))
}
