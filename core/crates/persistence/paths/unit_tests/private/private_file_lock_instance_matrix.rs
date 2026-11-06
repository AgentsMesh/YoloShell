use super::*;
use crate::paths::constants::PRIVATE_FILE_MODE;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const LOCK_MAX_BYTES: u64 = 64;
const LOCK_BYTES: &[u8] = b"lock-instance";
const HOOK_ERROR: &str = "injected private lock hook failure";
const CREATED_LOCK_NAME: &str = "created.lock";
const EXISTING_LOCK_NAME: &str = "existing.lock";
const MISSING_LOCK_NAME: &str = "missing.lock";
const INVALID_CREATE_LOCK_NAME: &str = "invalid-create.lock";
const INVALID_EXISTING_LOCK_NAME: &str = "invalid-existing.lock";
const BUSY_LOCK_NAME: &str = "busy.lock";
const HOOK_FAILURE_LOCK_NAME: &str = "hook-failure.lock";
const REPLACED_LOCK_NAME: &str = "replaced.lock";
const DISPLACED_LOCK_NAME: &str = "displaced.lock";

#[derive(Clone, Copy)]
enum Case {
    Create,
    Existing,
    MissingExisting,
    InvalidCreate,
    InvalidExisting,
    BusyCreate,
    HookFailure,
    PathReplacement,
}

impl Case {
    fn create(self) -> bool {
        matches!(
            self,
            Self::Create
                | Self::InvalidCreate
                | Self::BusyCreate
                | Self::HookFailure
                | Self::PathReplacement
        )
    }
}

#[test]
fn shared_lock_instance_covers_creation_absence_contention_and_failures() {
    let root = tempfile::tempdir().unwrap();

    let created = root.path().join(CREATED_LOCK_NAME);
    assert!(acquire(&created, Case::Create).unwrap().is_some());

    let existing = root.path().join(EXISTING_LOCK_NAME);
    write_private(&existing);
    assert!(acquire(&existing, Case::Existing).unwrap().is_some());

    let missing = root.path().join(MISSING_LOCK_NAME);
    assert!(acquire(&missing, Case::MissingExisting).unwrap().is_none());

    let invalid_create = root.path().join(INVALID_CREATE_LOCK_NAME);
    fs::create_dir(&invalid_create).unwrap();
    assert!(acquire(&invalid_create, Case::InvalidCreate).is_err());

    let invalid_existing = root.path().join(INVALID_EXISTING_LOCK_NAME);
    fs::create_dir(&invalid_existing).unwrap();
    assert!(acquire(&invalid_existing, Case::InvalidExisting).is_err());

    let busy = root.path().join(BUSY_LOCK_NAME);
    let held = PrivateFileLock::try_acquire(&busy, LOCK_MAX_BYTES)
        .unwrap()
        .unwrap();
    assert!(acquire(&busy, Case::BusyCreate).unwrap().is_none());
    drop(held);

    let hook_failure = root.path().join(HOOK_FAILURE_LOCK_NAME);
    assert!(acquire(&hook_failure, Case::HookFailure).is_err());

    let replaced = root.path().join(REPLACED_LOCK_NAME);
    assert!(acquire(&replaced, Case::PathReplacement).is_err());
}

fn acquire(path: &Path, case: Case) -> Result<Option<PrivateFileLock>> {
    PrivateFileLock::try_acquire_inner(path, LOCK_MAX_BYTES, case.create(), || match case {
        Case::HookFailure => anyhow::bail!(HOOK_ERROR),
        Case::PathReplacement => {
            fs::rename(path, path.with_file_name(DISPLACED_LOCK_NAME))?;
            write_private(path);
            Ok(())
        }
        _ => Ok(()),
    })
}

fn write_private(path: &Path) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    file.write_all(LOCK_BYTES).unwrap();
    file.sync_all().unwrap();
}
