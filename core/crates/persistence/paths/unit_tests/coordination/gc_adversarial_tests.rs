use super::gc_tests::candidate_path;
use super::*;
use crate::paths::constants::PRIVATE_FILE_MODE;
use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file::PrivateFileEntry;
use crate::paths::private_file_lock::PrivateFileLock;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt, PermissionsExt, symlink};

const TEST_ENTRY_BUDGET: usize = 4;
const TEST_DELETE_BUDGET: usize = 2;
const STALE_CANDIDATES: usize = TEST_ENTRY_BUDGET * 2 + 1;
const UNKNOWN_ENTRIES: usize = TEST_ENTRY_BUDGET + 1;
const ORIGINAL_BYTES: &[u8] = b"owned";
const REPLACEMENT_BYTES: &[u8] = b"replacement";
const UNSAFE_MODE: u32 = 0o644;
const FIFO_CREATE_SUCCESS: libc::c_int = 0;
const FILE_LOCK_SUCCESS: libc::c_int = 0;

fn root() -> (tempfile::TempDir, PrivateDirectory) {
    let temp = tempfile::tempdir().unwrap();
    let directory = PrivateDirectory::ensure(&temp.path().join("coordination")).unwrap();
    (temp, directory)
}

#[test]
fn unsafe_path_errors_are_preserved_but_resource_and_io_failures_propagate() {
    for code in [libc::ELOOP, libc::EISDIR, libc::ENOTDIR] {
        let error = anyhow::Error::new(std::io::Error::from_raw_os_error(code));
        assert!(scavenge::changed_or_unproven(&error));
    }
    for code in [libc::EIO, libc::EMFILE] {
        let error = anyhow::Error::new(std::io::Error::from_raw_os_error(code));
        assert!(!scavenge::changed_or_unproven(&error));
    }
}

#[test]
fn pre_unlink_open_cannot_pass_the_post_flock_path_identity_fence() {
    let (_temp, directory) = root();
    let path = candidate_path(directory.path(), 91);
    let owner = CoordinationLock::try_acquire(&path, MAX_COORDINATION_LOCK_BYTES)
        .unwrap()
        .unwrap();
    let entry = PrivateFileEntry::open(&path).unwrap();
    let stale = entry.open_rw(MAX_COORDINATION_LOCK_BYTES).unwrap();
    drop(owner);

    let result = unsafe { libc::flock(stale.file().as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    assert_eq!(result, FILE_LOCK_SUCCESS);
    let replacement = CoordinationLock::try_acquire(&path, MAX_COORDINATION_LOCK_BYTES)
        .unwrap()
        .unwrap();
    assert!(
        entry
            .verify_current_unchanged(stale.identity(), MAX_COORDINATION_LOCK_BYTES)
            .is_err()
    );
    drop(replacement);
}

#[test]
fn one_sweep_never_exceeds_its_entry_probe_or_delete_budget() {
    let (_temp, directory) = root();
    let locks: Vec<_> = (0..STALE_CANDIDATES)
        .map(|index| {
            let path = candidate_path(directory.path(), index);
            PrivateFileLock::try_acquire(&path, MAX_COORDINATION_LOCK_BYTES)
                .unwrap()
                .unwrap()
        })
        .collect();
    let work =
        scavenge::sweep_for_test(directory.path(), TEST_ENTRY_BUDGET, TEST_DELETE_BUDGET).unwrap();
    assert!(work.inspected <= TEST_ENTRY_BUDGET);
    assert!(work.probed <= TEST_ENTRY_BUDGET);
    assert!(work.deleted <= TEST_DELETE_BUDGET);
    drop(locks);
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}

#[test]
fn unlink_boundary_replacement_is_preserved() {
    let (_temp, directory) = root();
    let path = candidate_path(directory.path(), 0);
    write_private(&path, ORIGINAL_BYTES);
    let saved = directory.path().join("saved");
    let mut lock = PrivateFileLock::try_acquire_existing(&path, MAX_COORDINATION_LOCK_BYTES)
        .unwrap()
        .unwrap();
    let result = lock.unlink_owned_with_hook(|| {
        fs::rename(&path, &saved)?;
        write_private(&path, REPLACEMENT_BYTES);
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(path).unwrap(), REPLACEMENT_BYTES);
    assert_eq!(fs::read(saved).unwrap(), ORIGINAL_BYTES);
}

#[test]
fn unsafe_candidates_are_never_modified() {
    let (_temp, directory) = root();
    let hard_source = directory.path().join("hard-source");
    write_private(&hard_source, ORIGINAL_BYTES);
    let hard = candidate_path(directory.path(), 2);
    fs::hard_link(&hard_source, &hard).unwrap();
    let link = candidate_path(directory.path(), 3);
    symlink(&hard_source, &link).unwrap();
    let mode = candidate_path(directory.path(), 4);
    write_private(&mode, ORIGINAL_BYTES);
    fs::set_permissions(&mode, fs::Permissions::from_mode(UNSAFE_MODE)).unwrap();
    let directory_entry = candidate_path(directory.path(), 5);
    fs::create_dir(&directory_entry).unwrap();
    let fifo = candidate_path(directory.path(), 6);
    let fifo_name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    let fifo_mode = libc::mode_t::try_from(PRIVATE_FILE_MODE).unwrap();
    let result = unsafe { libc::mkfifo(fifo_name.as_ptr(), fifo_mode) };
    assert_eq!(result, FIFO_CREATE_SUCCESS);
    scavenge::sweep(directory.path()).unwrap();
    assert!(hard.exists());
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(fs::read(mode).unwrap(), ORIGINAL_BYTES);
    assert!(directory_entry.is_dir());
    assert!(fifo.symlink_metadata().unwrap().file_type().is_fifo());
}

#[test]
fn background_pass_converges_past_unknown_entries_and_delete_budget() {
    let (_temp, directory) = root();
    for index in 0..UNKNOWN_ENTRIES {
        write_private(
            &directory.path().join(format!("unknown-{index}")),
            ORIGINAL_BYTES,
        );
    }
    let candidates: Vec<_> = (0..STALE_CANDIDATES)
        .map(|index| {
            let path = candidate_path(directory.path(), index + UNKNOWN_ENTRIES);
            write_private(&path, ORIGINAL_BYTES);
            path
        })
        .collect();
    let work =
        scavenge::sweep_pass_for_test(directory.path(), TEST_ENTRY_BUDGET, TEST_DELETE_BUDGET)
            .unwrap();
    let remaining: Vec<_> = candidates
        .iter()
        .filter(|path| path.exists())
        .map(|path| path.file_name().unwrap().to_owned())
        .collect();
    assert!(
        remaining.is_empty(),
        "remaining={remaining:?} work=({}, {}, {})",
        work.inspected,
        work.probed,
        work.deleted
    );
    for index in 0..UNKNOWN_ENTRIES {
        assert!(directory.path().join(format!("unknown-{index}")).exists());
    }
}
