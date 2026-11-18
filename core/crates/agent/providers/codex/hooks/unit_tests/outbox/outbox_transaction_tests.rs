use super::{envelope, identity};
use crate::codex_hooks::CodexOutbox;
use crate::codex_hooks::constants::{LOCK_RECORD_MAX_BYTES, OUTBOX_FILE_NAME, OUTBOX_LOCK_SUFFIX};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use uuid::Uuid;

const INVOCATION_DIRECTORY: &str = "invocation";
const RETAINED_DIRECTORY: &str = "retained-invocation";
const FOREIGN_DIRECTORY: &str = "foreign";
const DISPLACED_FILE: &str = "displaced-outbox.json";
const DISPLACED_LOCK: &str = "displaced-outbox.lock";

struct Fixture {
    _root: TempDir,
    invocation: PathBuf,
    retained: PathBuf,
    outbox_path: PathBuf,
    foreign_path: PathBuf,
    original_bytes: Vec<u8>,
    foreign_bytes: Vec<u8>,
    outbox: CodexOutbox,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let invocation = root.path().join(INVOCATION_DIRECTORY);
        let retained = root.path().join(RETAINED_DIRECTORY);
        let outbox_path = invocation.join(OUTBOX_FILE_NAME);
        let foreign_path = root.path().join(FOREIGN_DIRECTORY).join(OUTBOX_FILE_NAME);
        let outbox = populated_outbox(&outbox_path);
        populated_outbox(&foreign_path);
        Self {
            original_bytes: std::fs::read(&outbox_path).unwrap(),
            foreign_bytes: std::fs::read(&foreign_path).unwrap(),
            _root: root,
            invocation,
            retained,
            outbox_path,
            foreign_path,
            outbox,
        }
    }

    fn directory_substitution(&self) -> impl FnOnce() + 'static {
        let invocation = self.invocation.clone();
        let retained = self.retained.clone();
        let outbox_path = self.outbox_path.clone();
        let foreign_path = self.foreign_path.clone();
        move || {
            std::fs::rename(invocation, retained).unwrap();
            crate::paths::ensure_private_parent(&outbox_path).unwrap();
            std::fs::rename(foreign_path, outbox_path).unwrap();
        }
    }

    fn assert_directory_substitution_was_contained(&self) {
        assert_eq!(
            std::fs::read(&self.outbox_path).unwrap(),
            self.foreign_bytes
        );
        assert_eq!(
            std::fs::read(self.retained.join(OUTBOX_FILE_NAME)).unwrap(),
            self.original_bytes
        );
    }
}

fn populated_outbox(path: &Path) -> CodexOutbox {
    let outbox = CodexOutbox::new(path).unwrap();
    let owner = identity();
    outbox.enqueue(envelope(&owner, Uuid::new_v4())).unwrap();
    outbox
}

#[test]
fn invocation_substitution_between_lock_and_load_fails_closed() {
    let fixture = Fixture::new();
    super::lock::set_after_acquire_hook(fixture.directory_substitution());
    assert!(fixture.outbox.load().is_err());
    fixture.assert_directory_substitution_was_contained();
}

#[test]
fn missing_outbox_does_not_hide_invocation_substitution() {
    let root = tempfile::tempdir().unwrap();
    let invocation = root.path().join(INVOCATION_DIRECTORY);
    let retained = root.path().join(RETAINED_DIRECTORY);
    let outbox_path = invocation.join(OUTBOX_FILE_NAME);
    let outbox = CodexOutbox::new(&outbox_path).unwrap();
    let replacement_path = outbox_path.clone();
    super::lock::set_after_acquire_hook(move || {
        std::fs::rename(invocation, retained).unwrap();
        crate::paths::ensure_private_parent(&replacement_path).unwrap();
    });

    assert!(outbox.load().is_err());
    assert!(!outbox_path.exists());
}

#[test]
fn invocation_substitution_between_load_and_persist_fails_closed() {
    let fixture = Fixture::new();
    super::outbox_storage::set_after_load_hook(fixture.directory_substitution());
    let owner = identity();
    assert!(
        fixture
            .outbox
            .enqueue(envelope(&owner, Uuid::new_v4()))
            .is_err()
    );
    fixture.assert_directory_substitution_was_contained();
}

#[test]
fn target_substitution_between_load_and_persist_fails_closed() {
    let fixture = Fixture::new();
    let displaced = fixture.invocation.join(DISPLACED_FILE);
    let outbox_path = fixture.outbox_path.clone();
    let foreign_path = fixture.foreign_path.clone();
    let displaced_for_hook = displaced.clone();
    super::outbox_storage::set_after_load_hook(move || {
        std::fs::rename(&outbox_path, displaced_for_hook).unwrap();
        std::fs::rename(foreign_path, outbox_path).unwrap();
    });
    let owner = identity();
    assert!(
        fixture
            .outbox
            .enqueue(envelope(&owner, Uuid::new_v4()))
            .is_err()
    );
    assert_eq!(
        std::fs::read(&fixture.outbox_path).unwrap(),
        fixture.foreign_bytes
    );
    assert_eq!(std::fs::read(displaced).unwrap(), fixture.original_bytes);
}

#[test]
fn target_disappearance_during_load_is_not_treated_as_an_empty_outbox() {
    let fixture = Fixture::new();
    let path = fixture.outbox_path.clone();
    super::outbox_storage::set_before_read_hook(move || {
        std::fs::remove_file(path).unwrap();
    });

    assert!(fixture.outbox.load().is_err());
}

#[test]
fn target_disappearance_after_open_is_not_treated_as_an_empty_outbox() {
    let fixture = Fixture::new();
    let path = fixture.outbox_path.clone();
    super::outbox_storage::set_after_open_hook(move || {
        std::fs::remove_file(path).unwrap();
    });

    assert!(fixture.outbox.load().is_err());
}

#[cfg(unix)]
#[test]
fn lock_replacement_after_acquire_is_rejected_before_load() {
    let fixture = Fixture::new();
    let lock_path = lock_path(&fixture.outbox_path);
    let displaced = fixture.outbox_path.parent().unwrap().join(DISPLACED_LOCK);
    let replacement = lock_path.clone();
    let displaced_for_hook = displaced.clone();
    let replacement_for_hook = replacement.clone();
    super::lock::set_after_acquire_hook(move || {
        std::fs::rename(&lock_path, &displaced_for_hook).unwrap();
        let entry =
            crate::paths::private_file::PrivateFileEntry::ensure(&replacement_for_hook).unwrap();
        drop(entry.open_rw_or_create(LOCK_RECORD_MAX_BYTES).unwrap());
    });

    assert!(fixture.outbox.load().is_err());
    assert!(displaced.exists());
    assert!(replacement.exists());
}

fn lock_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(OUTBOX_LOCK_SUFFIX);
    PathBuf::from(value)
}
