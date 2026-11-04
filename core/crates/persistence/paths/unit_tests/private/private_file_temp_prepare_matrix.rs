use super::*;
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const TARGET_NAME: &str = "prepare-matrix.bin";
const ACTIVE_NAME: &str = ".prepare-active";
const SLOT_NAME: &str = ".prepare-slot";
const SAVED_NAME: &str = ".prepare-saved";
const REFERENCE_NAME: &str = ".prepare-reference";
const ORIGINAL_BYTES: &[u8] = b"prepare-original";
const REPLACEMENT_BYTES: &[u8] = b"prepare-replacement";
const CLOSE_SUCCESS: libc::c_int = 0;
const INVALID_DIRECTORY_DESCRIPTOR: std::os::fd::RawFd = -1;

#[test]
fn cleanup_io_rejects_invalid_directory_descriptors() {
    assert!(private_file_cleanup_io::DirectoryStream::open(INVALID_DIRECTORY_DESCRIPTOR).is_err());
    assert!(private_file_cleanup_io::lock_directory(INVALID_DIRECTORY_DESCRIPTOR).is_err());
}

#[test]
fn unsafe_recycled_slot_errors_without_following_it() {
    let (_root, entry) = fixture();
    let slot = name(SLOT_NAME);
    let slot_path = named_path(&entry, &slot);
    fs::create_dir(&slot_path).unwrap();

    let result = entry.prepare_active(
        &name(ACTIVE_NAME),
        &slot,
        &mut || Ok(()),
        &mut || Ok(()),
        &mut || Ok(()),
    );

    assert!(result.is_err());
    assert!(slot_path.is_dir());
}

#[test]
fn active_and_recycled_slot_lock_contention_request_a_retry() {
    for source in [Source::Active, Source::Slot] {
        let (_root, entry) = fixture();
        let active = name(ACTIVE_NAME);
        let slot = name(SLOT_NAME);
        let path = named_path(&entry, source.name(&active, &slot));
        write_private(&path, ORIGINAL_BYTES);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let owner = private_file_temp_prepare::lock_or_retry(&file)
            .unwrap()
            .unwrap();

        let prepared = entry
            .prepare_active(&active, &slot, &mut || Ok(()), &mut || Ok(()), &mut || {
                Ok(())
            })
            .unwrap();

        assert!(prepared.is_none());
        assert_eq!(fs::read(path).unwrap(), ORIGINAL_BYTES);
        drop(owner);
    }
}

#[test]
fn opened_name_replacement_is_never_truncated() {
    for source in [Source::Active, Source::Slot] {
        let (root, entry) = fixture();
        let active = name(ACTIVE_NAME);
        let slot = name(SLOT_NAME);
        let path = named_path(&entry, source.name(&active, &slot));
        let saved = root.path().join(SAVED_NAME);
        write_private(&path, ORIGINAL_BYTES);

        let prepared = entry
            .prepare_active(
                &active,
                &slot,
                &mut || {
                    fs::rename(&path, &saved)?;
                    write_private(&path, REPLACEMENT_BYTES);
                    Ok(())
                },
                &mut || Ok(()),
                &mut || Ok(()),
            )
            .unwrap();

        assert!(prepared.is_none());
        assert_eq!(fs::read(saved).unwrap(), ORIGINAL_BYTES);
        assert_eq!(fs::read(path).unwrap(), REPLACEMENT_BYTES);
    }
}

#[test]
fn unsafe_identity_lookup_and_invalid_lock_descriptor_propagate() {
    let (root, entry) = fixture();
    let active = name(ACTIVE_NAME);
    fs::create_dir(named_path(&entry, &active)).unwrap();
    let reference = root.path().join(REFERENCE_NAME);
    write_private(&reference, ORIGINAL_BYTES);
    let reference_file = File::open(&reference).unwrap();
    let identity =
        FileIdentity::inspect_bounded(&reference_file.metadata().unwrap(), &reference, u64::MAX)
            .unwrap();
    assert!(entry.active_identity_matches(&active, identity).is_err());

    let invalid = File::open(&reference).unwrap();
    let descriptor = invalid.as_raw_fd();
    assert_eq!(unsafe { libc::close(descriptor) }, CLOSE_SUCCESS);
    let result = private_file_temp_prepare::lock_or_retry(&invalid);
    std::mem::forget(invalid);
    assert!(result.is_err());
}

#[derive(Clone, Copy)]
enum Source {
    Active,
    Slot,
}

impl Source {
    fn name<'a>(self, active: &'a CString, slot: &'a CString) -> &'a CString {
        match self {
            Self::Active => active,
            Self::Slot => slot,
        }
    }
}

fn fixture() -> (tempfile::TempDir, PrivateFileEntry) {
    let root = tempfile::tempdir().unwrap();
    let entry = PrivateFileEntry::ensure(&root.path().join(TARGET_NAME)).unwrap();
    (root, entry)
}

fn name(value: &str) -> CString {
    CString::new(value).unwrap()
}

fn named_path(entry: &PrivateFileEntry, name: &std::ffi::CStr) -> PathBuf {
    entry.path().with_file_name(name.to_string_lossy().as_ref())
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

#[path = "private_file_temp_prepare_branch_tests.rs"]
mod branch_tests;
