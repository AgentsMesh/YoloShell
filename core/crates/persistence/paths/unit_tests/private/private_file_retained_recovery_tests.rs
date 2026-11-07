use super::*;
use crate::paths::constants::PERMISSION_MODE_MASK;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink};

const TARGET_NAME: &str = "retained-recovery.bin";
const ISOLATED_BYTES: &[u8] = b"isolated-retained-content";
const OCCUPANT_BYTES: &[u8] = b"occupied-destination-content";
const REPLACEMENT_BYTES: &[u8] = b"replacement-isolated-content";
const SYMLINK_SOURCE_NAME: &str = "retained-symlink-source";
const UNSAFE_FILE_MODE: u32 = 0o644;

#[test]
fn valid_isolated_retained_state_restores_or_recycles_by_operation() {
    for operation in [RecoveryOperation::Restore, RecoveryOperation::Recycle] {
        let fixture = Fixture::new();
        fixture.write_isolated(ISOLATED_BYTES, PRIVATE_FILE_MODE);
        let opened = fixture
            .entry
            .open_isolated_retained(u64::MAX)
            .unwrap()
            .unwrap();
        let identity = opened.identity;
        drop(opened);

        operation.run(&fixture.entry, identity).unwrap();

        assert!(!fixture.isolated.exists());
        assert_eq!(
            fs::read(operation.destination(&fixture)).unwrap(),
            ISOLATED_BYTES
        );
    }
}

#[test]
fn missing_and_unsafe_isolated_states_fail_closed() {
    let missing = Fixture::new();
    assert!(
        missing
            .entry
            .open_isolated_retained(u64::MAX)
            .unwrap()
            .is_none()
    );

    for shape in [UnsafeShape::Symlink, UnsafeShape::WrongMode] {
        let fixture = Fixture::new();
        let source = fixture.root.path().join(SYMLINK_SOURCE_NAME);
        match shape {
            UnsafeShape::Symlink => {
                write_private(&source, ISOLATED_BYTES, PRIVATE_FILE_MODE);
                symlink(&source, &fixture.isolated).unwrap();
            }
            UnsafeShape::WrongMode => {
                fixture.write_isolated(ISOLATED_BYTES, UNSAFE_FILE_MODE);
            }
        }

        assert!(fixture.entry.open_isolated_retained(u64::MAX).is_err());
        if matches!(shape, UnsafeShape::Symlink) {
            assert_eq!(fs::read(source).unwrap(), ISOLATED_BYTES);
            assert!(
                fs::symlink_metadata(&fixture.isolated)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        } else {
            assert_eq!(fs::read(&fixture.isolated).unwrap(), ISOLATED_BYTES);
            assert_eq!(
                fs::symlink_metadata(&fixture.isolated)
                    .unwrap()
                    .permissions()
                    .mode()
                    & PERMISSION_MODE_MASK,
                UNSAFE_FILE_MODE
            );
        }
    }
}

#[test]
fn identity_mismatch_preserves_the_replacement_for_every_recovery_operation() {
    for operation in [RecoveryOperation::Restore, RecoveryOperation::Recycle] {
        let fixture = Fixture::new();
        fixture.write_isolated(ISOLATED_BYTES, PRIVATE_FILE_MODE);
        let opened = fixture
            .entry
            .open_isolated_retained(u64::MAX)
            .unwrap()
            .unwrap();
        let expected = opened.identity;
        drop(opened);
        fs::remove_file(&fixture.isolated).unwrap();
        fixture.write_isolated(REPLACEMENT_BYTES, PRIVATE_FILE_MODE);

        assert!(operation.run(&fixture.entry, expected).is_err());
        assert_eq!(fs::read(&fixture.isolated).unwrap(), REPLACEMENT_BYTES);
        assert!(!operation.destination(&fixture).exists());
    }
}

#[test]
fn occupied_destination_preserves_both_files_for_every_recovery_operation() {
    for operation in [RecoveryOperation::Restore, RecoveryOperation::Recycle] {
        let fixture = Fixture::new();
        fixture.write_isolated(ISOLATED_BYTES, PRIVATE_FILE_MODE);
        let opened = fixture
            .entry
            .open_isolated_retained(u64::MAX)
            .unwrap()
            .unwrap();
        let expected = opened.identity;
        drop(opened);
        let destination = operation.destination(&fixture);
        write_private(&destination, OCCUPANT_BYTES, PRIVATE_FILE_MODE);

        assert!(operation.run(&fixture.entry, expected).is_err());
        assert_eq!(fs::read(&fixture.isolated).unwrap(), ISOLATED_BYTES);
        assert_eq!(fs::read(destination).unwrap(), OCCUPANT_BYTES);
    }
}

#[derive(Clone, Copy)]
enum RecoveryOperation {
    Restore,
    Recycle,
}

impl RecoveryOperation {
    fn run(self, entry: &PrivateFileEntry, expected: FileIdentity) -> anyhow::Result<()> {
        match self {
            Self::Restore => entry.restore_isolated_retained(expected, u64::MAX),
            Self::Recycle => entry.recycle_isolated_retained(expected, u64::MAX),
        }
    }

    fn destination(self, fixture: &Fixture) -> PathBuf {
        match self {
            Self::Restore => fixture.target.clone(),
            Self::Recycle => fixture.retired.clone(),
        }
    }
}

#[derive(Clone, Copy)]
enum UnsafeShape {
    Symlink,
    WrongMode,
}

struct Fixture {
    root: tempfile::TempDir,
    target: PathBuf,
    isolated: PathBuf,
    retired: PathBuf,
    entry: PrivateFileEntry,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join(TARGET_NAME);
        let entry = PrivateFileEntry::ensure(&target).unwrap();
        let isolated_name = private_file_names::retained_isolation_name(entry.file_name()).unwrap();
        let retired_name = private_file_names::retired_name(entry.file_name()).unwrap();
        Self {
            isolated: named_path(&target, &isolated_name),
            retired: named_path(&target, &retired_name),
            root,
            target,
            entry,
        }
    }

    fn write_isolated(&self, bytes: &[u8], mode: u32) {
        write_private(&self.isolated, bytes, mode);
    }
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

include!("private_file_retained_recovery_branch_tests.rs");
