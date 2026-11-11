use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;

#[path = "../test_support/idle_observation_fixture.rs"]
mod fixture;
use fixture::*;

#[test]
fn unchanged_files_are_observed_without_consuming_the_validation() {
    let fixture = Fixture::new();
    let observed = fixture.watch();
    assert!(observed.finish_validation().unwrap());
    assert!(observed.unchanged().unwrap());
    assert!(observed.unchanged().unwrap());
}

#[test]
fn changing_either_custody_or_runtime_record_invalidates_the_shared_observation() {
    for runtime_changed in [false, true] {
        let fixture = Fixture::new();
        let runtime = fixture.parent.join(RUNTIME_NAME);
        std::fs::write(&runtime, CONTENT).unwrap();
        let observed =
            KernelObservation::watch(&[fixture.file.clone(), runtime.clone()], Vec::new()).unwrap();
        assert!(observed.finish_validation().unwrap());
        std::fs::write(
            if runtime_changed {
                runtime
            } else {
                fixture.file.clone()
            },
            CONTENT,
        )
        .unwrap();
        assert!(!observed.unchanged().unwrap());
    }
}

#[test]
fn file_write_and_same_value_atomic_replacement_invalidate() {
    for replacement in [false, true] {
        let fixture = Fixture::new();
        let observed = fixture.watch();
        assert!(observed.finish_validation().unwrap());
        if replacement {
            let next = fixture.file.with_extension(REPLACEMENT_EXTENSION);
            std::fs::write(&next, CONTENT).unwrap();
            std::fs::rename(next, &fixture.file).unwrap();
        } else {
            std::fs::write(&fixture.file, CONTENT).unwrap();
        }
        assert!(!observed.unchanged().unwrap());
    }
}

#[test]
fn replacement_between_registration_and_full_validation_cannot_be_cached() {
    let fixture = Fixture::new();
    let observed = fixture.watch();
    let moved = fixture.file.with_extension(REPLACEMENT_EXTENSION);
    std::fs::rename(&fixture.file, &moved).unwrap();
    std::fs::write(&fixture.file, CONTENT).unwrap();
    assert!(!observed.finish_validation().unwrap());
}

#[test]
fn ancestor_rename_and_rename_back_remain_observable() {
    let fixture = Fixture::new();
    let observed = fixture.watch();
    assert!(observed.finish_validation().unwrap());
    let moved = fixture.parent.with_extension(REPLACEMENT_EXTENSION);
    std::fs::rename(&fixture.parent, &moved).unwrap();
    std::fs::rename(&moved, &fixture.parent).unwrap();
    assert!(!observed.unchanged().unwrap());
}

#[test]
fn ancestor_permissions_and_acl_changes_invalidate() {
    for acl in [false, true] {
        let fixture = Fixture::new();
        let observed = fixture.watch();
        assert!(observed.finish_validation().unwrap());
        if acl {
            assert!(
                std::process::Command::new(CHMOD)
                    .args([ADD_ACL, TEST_ACL])
                    .arg(&fixture.parent)
                    .status()
                    .unwrap()
                    .success()
            );
        } else {
            std::fs::set_permissions(
                &fixture.parent,
                std::fs::Permissions::from_mode(ALTERED_MODE),
            )
            .unwrap();
        }
        assert!(!observed.unchanged().unwrap());
    }
}

#[test]
fn deletion_hard_link_and_symlink_replacement_invalidate() {
    for mutation in [
        FileMutation::Delete,
        FileMutation::HardLink,
        FileMutation::Symlink,
    ] {
        let fixture = Fixture::new();
        let observed = fixture.watch();
        assert!(observed.finish_validation().unwrap());
        let alternate = fixture.file.with_extension(REPLACEMENT_EXTENSION);
        match mutation {
            FileMutation::Delete => std::fs::remove_file(&fixture.file).unwrap(),
            FileMutation::HardLink => std::fs::hard_link(&fixture.file, alternate).unwrap(),
            FileMutation::Symlink => {
                std::fs::rename(&fixture.file, &alternate).unwrap();
                symlink(alternate, &fixture.file).unwrap();
            }
        }
        assert!(!observed.unchanged().unwrap());
    }
}

#[test]
fn missing_or_symlink_observation_target_cannot_arm() {
    let fixture = Fixture::new();
    let absent = fixture.file.with_extension(REPLACEMENT_EXTENSION);
    assert!(KernelObservation::watch(&[absent.clone()], Vec::new()).is_err());
    symlink(&fixture.file, &absent).unwrap();
    assert!(KernelObservation::watch(&[absent], Vec::new()).is_err());
}

#[test]
fn unrelated_directory_entries_do_not_force_full_validation() {
    let fixture = Fixture::new();
    let observed = fixture.watch();
    assert!(observed.finish_validation().unwrap());
    let unrelated = fixture.parent.join(UNRELATED_NAME);
    std::fs::write(unrelated, CONTENT).unwrap();
    assert!(observed.unchanged().unwrap());
}

#[test]
fn process_exit_invalidates_the_armed_observation() {
    let fixture = Fixture::new();
    let mut child = Child::start();
    let identity = crate::process::ProcessIdentity::capture(child.0.id()).unwrap();
    let observed = KernelObservation::watch(
        &[fixture.file.clone()],
        vec![CustodyHolderIdentity {
            pid: identity.pid,
            start_time: identity.start_time,
        }],
    )
    .unwrap();
    assert!(observed.finish_validation().unwrap());
    child.stop();
    assert!(!observed.unchanged().unwrap());
}

enum FileMutation {
    Delete,
    HardLink,
    Symlink,
}
