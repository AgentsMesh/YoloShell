use super::*;
use crate::paths::constants::PERMISSION_MODE_MASK;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt, symlink};

const TARGET_NAME: &str = "publish-safety.bin";
const STAGE_PREFIX: &str = ".yolo-publish-safety-";
const STAGE_SUFFIX: &str = ".tmp";
const ORIGINAL_BYTES: &[u8] = b"original-private-content";
const STAGED_BYTES: &[u8] = b"staged-private-content";
const REPLACEMENT_BYTES: &[u8] = b"replacement-private-content";
const SAVED_STAGED_NAME: &str = "saved-staged-content";
const SYMLINK_SOURCE_NAME: &str = "symlink-source";
const UNSAFE_FILE_MODE: u32 = 0o644;
const EMPTY_FILE_LENGTH: u64 = 0;

#[test]
fn exchange_publish_preserves_a_post_commit_target_replacement() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL_BYTES, PRIVATE_FILE_MODE);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let temporary = prepared_temporary(&entry, &target);
    let stage = named_path(&target, &temporary.name);
    let saved = root.path().join(SAVED_STAGED_NAME);

    let result = entry.rename_temporary_with_hooks(
        &temporary,
        u64::MAX,
        || Ok(()),
        || {
            fs::rename(&target, &saved).unwrap();
            write_private(&target, REPLACEMENT_BYTES, PRIVATE_FILE_MODE);
        },
    );

    assert!(result.is_err());
    assert_eq!(fs::read(&target).unwrap(), REPLACEMENT_BYTES);
    assert_eq!(fs::read(&saved).unwrap(), STAGED_BYTES);
    assert_eq!(fs::read(&stage).unwrap(), ORIGINAL_BYTES);
}

#[test]
fn exclusive_install_preserves_replacement_when_rollback_slot_is_occupied() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let temporary = prepared_temporary(&entry, &target);
    let stage = named_path(&target, &temporary.name);
    let saved = root.path().join(SAVED_STAGED_NAME);

    let result = entry.rename_temporary_with_hooks(
        &temporary,
        u64::MAX,
        || Ok(()),
        || {
            fs::rename(&target, &saved).unwrap();
            write_private(&target, REPLACEMENT_BYTES, PRIVATE_FILE_MODE);
            write_private(&stage, ORIGINAL_BYTES, PRIVATE_FILE_MODE);
        },
    );

    assert!(result.is_err());
    assert_eq!(fs::read(target).unwrap(), REPLACEMENT_BYTES);
    assert_eq!(fs::read(saved).unwrap(), STAGED_BYTES);
    assert_eq!(fs::read(stage).unwrap(), ORIGINAL_BYTES);
}

#[test]
fn busy_active_stage_exhausts_bounded_retries_without_mutation_then_recovers() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let mut held = entry.temporary(STAGE_PREFIX, STAGE_SUFFIX).unwrap();
    held.file.write_all(STAGED_BYTES).unwrap();
    held.file.sync_all().unwrap();
    let stage = named_path(&target, &held.name);
    let identity = fs::symlink_metadata(&stage).unwrap();

    assert!(entry.temporary(STAGE_PREFIX, STAGE_SUFFIX).is_err());
    assert_eq!(fs::read(&stage).unwrap(), STAGED_BYTES);
    assert_eq!(fs::symlink_metadata(&stage).unwrap().ino(), identity.ino());

    entry.remove_temporary(&held).unwrap();
    drop(held);
    let recovered = entry.temporary(STAGE_PREFIX, STAGE_SUFFIX).unwrap();
    assert_eq!(recovered.file.metadata().unwrap().len(), EMPTY_FILE_LENGTH);
    entry.remove_temporary(&recovered).unwrap();
}

#[test]
fn unsafe_active_stage_shapes_fail_closed_without_following_or_replacing_them() {
    for shape in [
        UnsafeShape::Symlink,
        UnsafeShape::WrongMode,
        UnsafeShape::Directory,
    ] {
        let root = private_root();
        let target = root.path().join(TARGET_NAME);
        let entry = PrivateFileEntry::ensure(&target).unwrap();
        let active =
            private_file_names::active_name(entry.file_name(), STAGE_PREFIX, STAGE_SUFFIX).unwrap();
        let active_path = named_path(&target, &active);
        let source = root.path().join(SYMLINK_SOURCE_NAME);
        match shape {
            UnsafeShape::Symlink => {
                write_private(&source, ORIGINAL_BYTES, PRIVATE_FILE_MODE);
                symlink(&source, &active_path).unwrap();
            }
            UnsafeShape::WrongMode => {
                write_private(&active_path, ORIGINAL_BYTES, UNSAFE_FILE_MODE);
            }
            UnsafeShape::Directory => fs::create_dir(&active_path).unwrap(),
        }

        assert!(entry.temporary(STAGE_PREFIX, STAGE_SUFFIX).is_err());
        match shape {
            UnsafeShape::Symlink => {
                assert!(
                    fs::symlink_metadata(&active_path)
                        .unwrap()
                        .file_type()
                        .is_symlink()
                );
                assert_eq!(fs::read(source).unwrap(), ORIGINAL_BYTES);
            }
            UnsafeShape::WrongMode => {
                assert_eq!(fs::read(&active_path).unwrap(), ORIGINAL_BYTES);
                assert_eq!(
                    fs::symlink_metadata(active_path)
                        .unwrap()
                        .permissions()
                        .mode()
                        & PERMISSION_MODE_MASK,
                    UNSAFE_FILE_MODE
                );
            }
            UnsafeShape::Directory => assert!(active_path.is_dir()),
        }
    }
}

#[derive(Clone, Copy)]
enum UnsafeShape {
    Symlink,
    WrongMode,
    Directory,
}

fn prepared_temporary(entry: &PrivateFileEntry, target: &Path) -> TemporaryPrivateFile {
    let mut temporary = entry.temporary(STAGE_PREFIX, STAGE_SUFFIX).unwrap();
    temporary.file.write_all(STAGED_BYTES).unwrap();
    temporary.sync_and_inspect(target, u64::MAX).unwrap();
    temporary
}

fn private_root() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn write_private(path: &Path, bytes: &[u8], mode: u32) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(mode)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(mode))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn named_path(target: &Path, name: &std::ffi::CStr) -> PathBuf {
    target.with_file_name(OsStr::from_bytes(name.to_bytes()))
}

include!("private_file_publish_rollback_tests.rs");
