use super::*;
use std::{path::Path, process::Command};

const DIRECTORY_ENV: &str = "YOLO_DIRECTORY_LEASE_TEST_PATH";
const EXPECTED_ENV: &str = "YOLO_DIRECTORY_LEASE_TEST_EXCLUSIVE";
const SHARED: &str = "shared";
const EXCLUSIVE: &str = "exclusive";
const HELPER: &str = stringify!(socket_namespace_directory_lease_subprocess_helper);

fn probe(directory: &PrivateDirectory, expected: &str) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg(HELPER)
        .arg(crate::paths::constants::PROCESS_FILE_LOCK_TEST_NOCAPTURE_ARGUMENT)
        .env(DIRECTORY_ENV, directory.path())
        .env(EXPECTED_ENV, expected);
    let mut child = crate::process::spawn_waitable_test_child(&mut command);
    assert!(child.wait().unwrap().success());
}

#[test]
fn every_shared_participant_fences_same_process_and_other_process_cleanup() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = PrivateDirectory::ensure(temporary.path()).unwrap();
    let first = DirectoryLease::shared(&directory).unwrap();
    let second = DirectoryLease::shared(&directory).unwrap();
    assert!(DirectoryLease::try_exclusive(&directory).unwrap().is_none());
    probe(&directory, SHARED);
    drop(first);
    assert!(DirectoryLease::try_exclusive(&directory).unwrap().is_none());
    probe(&directory, SHARED);
    drop(second);
    probe(&directory, EXCLUSIVE);
    assert!(DirectoryLease::try_exclusive(&directory).unwrap().is_some());
}

#[test]
fn socket_namespace_directory_lease_subprocess_helper() {
    let Some(path) = std::env::var_os(DIRECTORY_ENV) else {
        return;
    };
    let directory = PrivateDirectory::open(Path::new(&path)).unwrap();
    let lease = DirectoryLease::try_exclusive(&directory).unwrap();
    match std::env::var(EXPECTED_ENV).unwrap().as_str() {
        SHARED => assert!(lease.is_none()),
        EXCLUSIVE => assert!(lease.is_some()),
        _ => panic!("unexpected directory lease probe mode"),
    }
}
