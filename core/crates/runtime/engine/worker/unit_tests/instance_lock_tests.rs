use super::*;
use std::os::unix::fs::OpenOptionsExt;

const STALE_LOCK_MODE: u32 = 0o600;

fn lock_fixture() -> (tempfile::TempDir, Paths, Uuid) {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    (directory, paths, terminal_id)
}

fn lock_path(paths: &Paths, terminal_id: Uuid) -> std::path::PathBuf {
    crate::coordination::lock_path(
        crate::coordination::CoordinationScope::Worker,
        paths.home(),
        Some(&terminal_id.to_string()),
    )
    .unwrap()
}

#[test]
fn instance_lock_is_exclusive_and_reuses_the_stale_lock_file() {
    let (_directory, paths, terminal_id) = lock_fixture();
    let lock_path = lock_path(&paths, terminal_id);
    drop(
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(STALE_LOCK_MODE)
            .open(lock_path)
            .unwrap(),
    );
    let first = WorkerInstanceLock::acquire(&paths, terminal_id).unwrap();
    let error = WorkerInstanceLock::acquire(&paths, terminal_id)
        .err()
        .unwrap();
    assert!(error.to_string().contains(WORKER_INSTANCE_BUSY_CONTEXT));
    drop(first);
    assert!(WorkerInstanceLock::acquire(&paths, terminal_id).is_ok());
}

#[test]
fn home_replacement_cannot_create_a_second_worker_lock() {
    let (directory, paths, terminal_id) = lock_fixture();
    let displaced = directory.path().with_extension("displaced");
    let first = WorkerInstanceLock::acquire(&paths, terminal_id).unwrap();
    std::fs::rename(directory.path(), &displaced).unwrap();
    paths.ensure_session(terminal_id).unwrap();
    let error = WorkerInstanceLock::acquire(&paths, terminal_id)
        .err()
        .unwrap();
    assert!(error.to_string().contains(WORKER_INSTANCE_BUSY_CONTEXT));
    drop(first);
    assert!(WorkerInstanceLock::acquire(&paths, terminal_id).is_ok());
    std::fs::remove_dir_all(displaced).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn coordination_parent_replacement_cannot_create_a_second_worker_lock() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("coordination");
    let displaced = root.path().join("displaced-coordination");
    crate::paths::directory::PrivateDirectory::ensure(&original).unwrap();
    let path = original.join(format!("worker-{}.lock", Uuid::new_v4()));
    let first = WorkerInstanceLock::acquire_path(&path).unwrap();
    std::fs::rename(&original, &displaced).unwrap();
    crate::paths::directory::PrivateDirectory::ensure(&original).unwrap();

    let error = WorkerInstanceLock::acquire_path(&path).err().unwrap();
    assert!(error.to_string().contains(WORKER_INSTANCE_BUSY_CONTEXT));
    drop(first);
    assert!(WorkerInstanceLock::acquire_path(&path).is_ok());
}
