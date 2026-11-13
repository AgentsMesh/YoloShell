use super::{capture, content, from_live, strict};
use crate::paths::{MAX_DURABLE_JSON_BYTES, atomic_write_json, private_file::PrivateFileEntry};
use std::os::unix::fs::PermissionsExt;

const RECORD: &str = "record.json";
const FOREIGN: &str = "foreign.json";
const VALUE: &str = "original content";
const CHANGED: &str = "same object changed content";
const UNSAFE_MODE: u32 = 0o644;
const PERMISSION_BITS: u32 = 0o777;

#[test]
fn replaced_source_never_refreshes_a_stale_live_or_durable_token() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(RECORD);
    atomic_write_json(&path, &VALUE).unwrap();
    let entry = PrivateFileEntry::open(&path).unwrap();
    let live = entry.open_read(MAX_DURABLE_JSON_BYTES).unwrap().identity();
    let original = from_live(&entry, live).unwrap();
    atomic_write_json(&path, &VALUE).unwrap();
    assert!(from_live(&entry, live).is_err());
    assert!(strict(&entry, original).is_err());
    assert!(content(&entry, original).is_err());
    let replacement = capture(&entry).unwrap();
    assert_ne!(replacement.object_identity(), original.object_identity());
    let fresh = strict(&entry, replacement).unwrap();
    assert_eq!(content(&entry, replacement).unwrap(), fresh);
    assert_eq!(crate::paths::read_json::<String>(&path).unwrap(), VALUE);
}

#[test]
fn unavailable_private_file_cannot_produce_any_mutation_authority() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(RECORD);
    atomic_write_json(&path, &VALUE).unwrap();
    let entry = PrivateFileEntry::open(&path).unwrap();
    let live = entry.open_read(MAX_DURABLE_JSON_BYTES).unwrap().identity();
    let version = capture(&entry).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(UNSAFE_MODE)).unwrap();
    assert!(capture(&entry).is_err());
    assert!(from_live(&entry, live).is_err());
    assert!(strict(&entry, version).is_err());
    assert!(content(&entry, version).is_err());
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & PERMISSION_BITS,
        UNSAFE_MODE
    );
    std::fs::remove_file(&path).unwrap();
    assert!(capture(&entry).is_err());
    assert!(from_live(&entry, live).is_err());
    assert!(strict(&entry, version).is_err());
    assert!(content(&entry, version).is_err());
    assert!(!path.exists());
}

#[test]
fn content_version_checks_reject_foreign_proof_and_in_place_content_mutation() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(RECORD);
    let foreign = root.path().join(FOREIGN);
    atomic_write_json(&path, &VALUE).unwrap();
    atomic_write_json(&foreign, &VALUE).unwrap();
    let entry = PrivateFileEntry::open(&path).unwrap();
    let original = capture(&entry).unwrap();
    let foreign_entry = PrivateFileEntry::open(&foreign).unwrap();
    assert!(strict(&foreign_entry, original).is_err());
    assert!(content(&foreign_entry, original).is_err());
    std::fs::write(&path, serde_json::to_vec(CHANGED).unwrap()).unwrap();
    assert!(strict(&entry, original).is_err());
    assert!(content(&entry, original).is_err());
    assert_eq!(
        capture(&entry).unwrap().object_identity(),
        original.object_identity()
    );
    assert_eq!(crate::paths::read_json::<String>(&path).unwrap(), CHANGED);
    assert_eq!(crate::paths::read_json::<String>(&foreign).unwrap(), VALUE);
}
