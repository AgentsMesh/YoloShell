use super::tests_security_support::*;
use super::*;
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink};
use std::os::unix::net::UnixListener;

#[test]
fn special_unsafe_and_linked_entries_are_rejected_without_following() {
    let root = private_root();
    let source = root.path().join("source");
    write_private(&source, ORIGINAL_BYTES);

    let symbolic = root.path().join("symbolic");
    symlink(&source, &symbolic).unwrap();
    assert_rejected(&symbolic);
    assert_eq!(fs::read(&source).unwrap(), ORIGINAL_BYTES);

    let linked = root.path().join("linked");
    fs::hard_link(&source, &linked).unwrap();
    assert_rejected(&linked);
    assert_eq!(fs::read(&source).unwrap(), ORIGINAL_BYTES);
    fs::remove_file(&linked).unwrap();

    let unsafe_file = root.path().join("unsafe");
    write_private(&unsafe_file, ORIGINAL_BYTES);
    fs::set_permissions(&unsafe_file, fs::Permissions::from_mode(UNSAFE_FILE_MODE)).unwrap();
    assert_rejected(&unsafe_file);

    let fifo = root.path().join("fifo");
    let fifo_name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe {
            libc::mkfifo(
                fifo_name.as_ptr(),
                crate::paths::constants::PRIVATE_FILE_MODE as libc::mode_t,
            )
        },
        0
    );
    assert_rejected(&fifo);

    let socket = root.path().join("socket");
    let _listener = UnixListener::bind(&socket).unwrap();
    assert_rejected(&socket);

    let directory = root.path().join("directory");
    set_private_directory(&directory);
    assert_rejected(&directory);
}

#[test]
fn journal_and_snapshot_enforce_maximums_before_reading() {
    let root = private_root();
    let journal_path = root.path().join("oversized-journal");
    sparse_private(
        &journal_path,
        MAX_JOURNAL_DISK_BYTES + INITIAL_RETAINED_SEQUENCE,
    );
    assert!(Journal::open(&journal_path, Uuid::new_v4()).is_err());
    assert_eq!(
        fs::metadata(&journal_path).unwrap().len(),
        MAX_JOURNAL_DISK_BYTES + 1
    );

    let exact_journal = root.path().join("exact-journal");
    sparse_private(&exact_journal, MAX_JOURNAL_DISK_BYTES);
    assert!(matches!(
        Journal::open(&exact_journal, Uuid::new_v4()),
        Err(JournalError::InvalidFormat)
    ));
    assert!(matches!(
        Journal::open_with_max_bytes(
            root.path().join("too-large-policy"),
            Uuid::new_v4(),
            MAX_JOURNAL_DISK_BYTES + 1,
        ),
        Err(JournalError::DiskLimitExceeded)
    ));

    let oversized_snapshot = root.path().join("oversized-snapshot");
    sparse_private(
        &oversized_snapshot,
        MAX_SNAPSHOT_DISK_BYTES + INITIAL_RETAINED_SEQUENCE,
    );
    assert!(read_snapshot(&oversized_snapshot).is_err());
    let exact_snapshot = root.path().join("exact-snapshot");
    sparse_private(&exact_snapshot, MAX_SNAPSHOT_DISK_BYTES);
    assert!(matches!(
        read_snapshot(&exact_snapshot),
        Err(JournalError::InvalidFormat)
    ));
}

#[test]
fn ancestor_symlinks_are_rejected_without_external_creation() {
    let root = private_root();
    let outside = root.path().join("outside");
    set_private_directory(&outside);
    let alias = root.path().join("alias");
    symlink(&outside, &alias).unwrap();
    let journal = alias.join("journal");
    let snapshot = alias.join("snapshot");
    assert!(Journal::open(&journal, Uuid::new_v4()).is_err());
    assert!(write_snapshot(&snapshot, &sample_snapshot(ORIGINAL_BYTES)).is_err());
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
}

fn assert_rejected(path: &Path) {
    assert!(Journal::open(path, Uuid::new_v4()).is_err());
    assert!(read_snapshot(path).is_err());
}

fn sparse_private(path: &Path, length: u64) {
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_len(length).unwrap();
    file.sync_all().unwrap();
}
