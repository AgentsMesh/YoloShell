use super::*;
use crate::paths::directory::entry_c_string;
use std::{
    ffi::{OsStr, OsString},
    fs,
    path::Path,
};

const HOME_DIRECTORY: &str = "registered-home";

#[test]
fn certified_pending_scans_hold_root_lifecycle_and_retire_after_last_owner_removal() {
    let temporary = tempfile::tempdir_in(ISOLATED_TEST_SOCKET_PARENT).unwrap();
    let root = PrivateDirectory::ensure(temporary.path()).unwrap();
    let home = tempfile::tempdir().unwrap();
    let home_path = home.path().join(HOME_DIRECTORY);
    PrivateDirectory::ensure(&home_path).unwrap();
    let paths = Paths::new_with_socket_root(home_path, root.path().to_path_buf());
    let namespace = paths.retain_socket_namespace().unwrap();
    let directory = namespace.directory().unwrap();
    let target = socket_stage::Target::new(
        directory.path(),
        &entry_c_string(OsStr::new(HOST_SOCKET_FILE_NAME)).unwrap(),
    )
    .unwrap();
    // A released target lease is exactly what makes stage scanning recreate its
    // lifecycle file while trying to reclaim bookkeeping after an owner leaves.
    drop(socket_stage::Lease::acquire_immediate_for_test(&directory, &target).unwrap());
    let mut scanner = RootScanner::new(root.try_clone().unwrap());
    for _ in 0..MAX_PRIVATE_DIRECTORY_ENTRIES {
        scanner.scan_chunk().unwrap();
        if scanner.pending.is_some() {
            break;
        }
    }
    let pending = scanner.pending.as_ref().unwrap();
    pending.authority.verify().unwrap();
    let revision = root.revision().unwrap();
    let before = entries(directory.path());
    let lifecycle = lifecycle_lock::SocketNamespaceLifecycleLock::acquire(&root).unwrap();
    assert!(matches!(scanner.scan_chunk().unwrap(), ScanChunk::Busy));
    assert_eq!(entries(directory.path()), before);
    assert!(scanner.pending.is_some());
    drop(lifecycle);

    namespace.release().unwrap();
    assert!(!directory.path().exists());
    assert_ne!(root.revision().unwrap(), revision);
    // This must invalidate the pending scan without reading a vanished certificate
    // or creating another .stage.lock through its old retained directory.
    assert!(matches!(scanner.scan_chunk().unwrap(), ScanChunk::Progress));
    assert!(scanner.pending.is_none());
    assert!(!directory.path().exists());
    drop(scanner);
    assert!(!directory.path().exists());
}

fn entries(path: &Path) -> Vec<OsString> {
    let mut entries = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    entries.sort();
    entries
}
