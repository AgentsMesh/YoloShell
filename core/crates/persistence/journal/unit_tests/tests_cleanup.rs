use super::tests_security_support::*;
use super::*;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink};

#[test]
fn valid_crash_residues_are_removed_but_unrelated_names_are_preserved() {
    let root = private_root();
    let snapshot_residue = root.path().join(stale_name(SNAPSHOT_TEMPORARY_PREFIX));
    let journal_residue = root.path().join(stale_name(COMPACT_TEMPORARY_PREFIX));
    let unrelated = root.path().join(format!(
        "{SNAPSHOT_TEMPORARY_PREFIX}not-a-uuid{TEMPORARY_SUFFIX}"
    ));
    write_private(&snapshot_residue, ORIGINAL_BYTES);
    write_private(&journal_residue, ORIGINAL_BYTES);
    write_private(&unrelated, ORIGINAL_BYTES);

    let snapshot = root.path().join("snapshot");
    write_snapshot(&snapshot, &sample_snapshot(b"new")).unwrap();
    assert!(!snapshot_residue.exists());
    assert!(journal_residue.exists());
    assert!(unrelated.exists());

    let journal = root.path().join("journal");
    Journal::open(&journal, Uuid::new_v4()).unwrap();
    assert!(!journal_residue.exists());
    assert!(unrelated.exists());
}

#[test]
fn matching_symlink_hardlink_and_unsafe_residues_fail_closed() {
    for kind in [
        ResidueKind::Symlink,
        ResidueKind::Hardlink,
        ResidueKind::Unsafe,
    ] {
        let root = private_root();
        let outside = root.path().join("outside");
        write_private(&outside, ORIGINAL_BYTES);
        let residue = root.path().join(stale_name(SNAPSHOT_TEMPORARY_PREFIX));
        match kind {
            ResidueKind::Symlink => symlink(&outside, &residue).unwrap(),
            ResidueKind::Hardlink => fs::hard_link(&outside, &residue).unwrap(),
            ResidueKind::Unsafe => {
                write_private(&residue, ORIGINAL_BYTES);
                fs::set_permissions(&residue, fs::Permissions::from_mode(UNSAFE_FILE_MODE))
                    .unwrap();
            }
        }
        let target = root.path().join("snapshot");
        assert!(write_snapshot(&target, &sample_snapshot(b"new")).is_err());
        assert_eq!(fs::read(&outside).unwrap(), ORIGINAL_BYTES);
        assert!(fs::symlink_metadata(&residue).is_ok());
        assert!(!target.exists());
    }
}

#[test]
fn directory_lock_prevents_live_temporary_cleanup() {
    let root = private_root();
    let target = root.path().join("snapshot");
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let lock = entry
        .lock_and_cleanup(SNAPSHOT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)
        .unwrap();
    let active = root.path().join(stale_name(SNAPSHOT_TEMPORARY_PREFIX));
    write_private(&active, ORIGINAL_BYTES);
    assert!(write_snapshot(&target, &sample_snapshot(b"new")).is_err());
    assert!(active.exists());
    drop(lock);
    write_snapshot(&target, &sample_snapshot(b"new")).unwrap();
    assert!(!active.exists());
}

#[test]
fn stale_scan_counts_all_entries_and_fails_closed_above_the_limit() {
    let root = private_root();
    let unrelated_count = crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES - 1;
    create_unrelated(root.path(), unrelated_count);
    let target = root.path().join("snapshot");
    write_snapshot(&target, &sample_snapshot(ORIGINAL_BYTES)).unwrap();
    write_snapshot(&target, &sample_snapshot(b"at-limit")).unwrap();

    let overflow = root.path().join("unrelated-overflow");
    create_private(&overflow);
    assert!(write_snapshot(&target, &sample_snapshot(b"rejected")).is_err());
    assert!(overflow.exists());
    assert_eq!(read_snapshot(&target).unwrap().data, b"at-limit");
}

#[derive(Clone, Copy)]
enum ResidueKind {
    Symlink,
    Hardlink,
    Unsafe,
}

fn create_unrelated(directory: &Path, count: usize) {
    for index in 0..count {
        create_private(&directory.join(format!("unrelated-{index}")));
    }
}

fn create_private(path: &Path) {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
}
