use super::*;
use crate::paths::constants::MAX_DURABLE_JSON_BYTES;

const MANAGED_DIRECTORY: &str = "managed";
const DISPLACED_DIRECTORY: &str = "displaced";
const OTHER_DIRECTORY: &str = "other";
const TARGET_FILE: &str = "target.json";
const OTHER_FILE: &str = "other.json";
const ORIGINAL_VALUE: u8 = 1;
const NEXT_VALUE: u8 = 2;

#[test]
fn serialized_commit_rejects_parent_replacement_after_the_caller_fence() {
    let fixture = Fixture::new();
    crate::paths::atomic_write_json(&fixture.target, &ORIGINAL_VALUE).unwrap();
    let entry = PrivateFileEntry::open(&fixture.target).unwrap();
    let previous = entry.inspect(MAX_DURABLE_JSON_BYTES).unwrap();
    let lock = entry.lock_retained_parent().unwrap();
    let authority = entry.parent_lock_authority(&lock).unwrap();
    let managed = fixture.managed.clone();
    let displaced = fixture.root.path().join(DISPLACED_DIRECTORY);
    let result = crate::paths::write_entry_expected_serialized(
        PrivateFileEntry::open(&fixture.target).unwrap(),
        previous,
        &NEXT_VALUE,
        &authority,
        || Ok(()),
        || {
            std::fs::rename(&managed, &displaced)?;
            PrivateDirectory::ensure(&managed)?;
            Ok(())
        },
        |_| {},
    );
    assert!(result.is_err());
    assert!(!fixture.target.exists());
    assert_eq!(
        crate::paths::read_json::<u8>(&displaced.join(TARGET_FILE)).unwrap(),
        ORIGINAL_VALUE
    );
}

#[test]
fn parent_authority_rejects_a_different_or_replaced_parent() {
    let fixture = Fixture::new();
    let entry = PrivateFileEntry::open(&fixture.target).unwrap();
    let lock = entry.lock_retained_parent().unwrap();
    let authority = entry.parent_lock_authority(&lock).unwrap();
    let other = fixture.root.path().join(OTHER_DIRECTORY);
    PrivateDirectory::ensure(&other).unwrap();
    let foreign = PrivateFileEntry::open(&other.join(TARGET_FILE)).unwrap();
    assert!(authority.require_entry(&foreign).is_err());
    std::fs::rename(
        &fixture.managed,
        fixture.root.path().join(DISPLACED_DIRECTORY),
    )
    .unwrap();
    PrivateDirectory::ensure(&fixture.managed).unwrap();
    let replacement = PrivateFileEntry::open(&fixture.target).unwrap();
    assert!(authority.require_entry(&replacement).is_err());
}

#[test]
fn parent_authority_intentionally_serializes_another_entry_in_the_same_parent() {
    let fixture = Fixture::new();
    let entry = PrivateFileEntry::open(&fixture.target).unwrap();
    let lock = entry.lock_retained_parent().unwrap();
    let authority = entry.parent_lock_authority(&lock).unwrap();
    let other_path = fixture.managed.join(OTHER_FILE);
    crate::paths::write_entry_expected_serialized(
        PrivateFileEntry::open(&other_path).unwrap(),
        None,
        &NEXT_VALUE,
        &authority,
        || Ok(()),
        || Ok(()),
        |_| {},
    )
    .unwrap();
    assert_eq!(
        crate::paths::read_json::<u8>(&other_path).unwrap(),
        NEXT_VALUE
    );
}

struct Fixture {
    root: tempfile::TempDir,
    managed: std::path::PathBuf,
    target: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let managed = root.path().join(MANAGED_DIRECTORY);
        PrivateDirectory::ensure(&managed).unwrap();
        let target = managed.join(TARGET_FILE);
        Self {
            root,
            managed,
            target,
        }
    }
}
