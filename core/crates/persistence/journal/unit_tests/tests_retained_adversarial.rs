use super::tests_retained_support::{RetainedFixture, saved_path};
use super::tests_security_support::{REPLACEMENT_BYTES, replace_target};
use super::*;
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::path::Path;

const IDENTITY_TAMPER_BYTES: usize = 1;
const FIRST_TAMPER_BYTE_INDEX: usize = 0;
const IDENTITY_TAMPER_OFFSET: u64 = 0;
const IDENTITY_TAMPER_XOR_MASK: u8 = u8::MAX;
const SYSCALL_SUCCESS: libc::c_int = 0;

#[derive(Clone, Copy, Debug)]
enum RetainedTarget {
    Journal,
    Cursor,
}

#[derive(Clone, Copy, Debug)]
enum NamespaceMutation {
    Delete,
    Replace,
}

#[test]
fn retained_reconcile_fences_deleted_or_replaced_names() {
    for target in [RetainedTarget::Journal, RetainedTarget::Cursor] {
        for mutation in [NamespaceMutation::Delete, NamespaceMutation::Replace] {
            assert_mutation_is_fenced(target, mutation);
        }
    }
}

#[test]
fn retained_journal_fence_includes_ctime() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let journal = fixture.moved_journal.clone();
    let expected = fs::read(&journal).unwrap();
    crate::paths::test_support::set_retained_install_hook(move || {
        tamper_and_restore(&journal).unwrap();
        Ok(())
    });

    assert!(fixture.reconcile().is_err());

    assert_eq!(fs::read(&fixture.moved_journal).unwrap(), expected);
    fixture.assert_replacements_unchanged();
}

fn assert_mutation_is_fenced(target: RetainedTarget, mutation: NamespaceMutation) {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let path = target_path(&fixture, target).to_path_buf();
    match mutation {
        NamespaceMutation::Delete => fs::remove_file(&path).unwrap(),
        NamespaceMutation::Replace => replace_target(&path, &saved_path(&path)),
    }

    assert!(fixture.reconcile().is_err(), "{target:?} {mutation:?}");

    if matches!(mutation, NamespaceMutation::Replace) {
        assert_eq!(fs::read(path).unwrap(), REPLACEMENT_BYTES);
    }
    fixture.assert_replacements_unchanged();
}

fn target_path(fixture: &RetainedFixture, target: RetainedTarget) -> &Path {
    match target {
        RetainedTarget::Journal => &fixture.moved_journal,
        RetainedTarget::Cursor => &fixture.moved_cursor,
    }
}

fn tamper_and_restore(path: &Path) -> std::io::Result<()> {
    let before = fs::metadata(path)?;
    let file = OpenOptions::new().read(true).write(true).open(path)?;
    let mut original = [0_u8; IDENTITY_TAMPER_BYTES];
    file.read_exact_at(&mut original, IDENTITY_TAMPER_OFFSET)?;
    let changed = [original[FIRST_TAMPER_BYTE_INDEX] ^ IDENTITY_TAMPER_XOR_MASK];
    file.write_all_at(&changed, IDENTITY_TAMPER_OFFSET)?;
    file.sync_data()?;
    file.write_all_at(&original, IDENTITY_TAMPER_OFFSET)?;
    file.sync_data()?;
    restore_times(&file, &before)?;
    let after = fs::metadata(path)?;
    assert_eq!(
        (before.mtime(), before.mtime_nsec()),
        (after.mtime(), after.mtime_nsec())
    );
    assert_ne!(
        (before.ctime(), before.ctime_nsec()),
        (after.ctime(), after.ctime_nsec())
    );
    Ok(())
}

fn restore_times(file: &fs::File, metadata: &fs::Metadata) -> std::io::Result<()> {
    let times = [
        libc::timespec {
            tv_sec: metadata.atime(),
            tv_nsec: metadata.atime_nsec(),
        },
        libc::timespec {
            tv_sec: metadata.mtime(),
            tv_nsec: metadata.mtime_nsec(),
        },
    ];
    if unsafe { libc::futimens(file.as_raw_fd(), times.as_ptr()) } == SYSCALL_SUCCESS {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
