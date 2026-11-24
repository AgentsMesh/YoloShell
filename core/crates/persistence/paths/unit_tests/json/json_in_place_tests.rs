use super::super::*;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::{FileExt, OpenOptionsExt, PermissionsExt};
use tempfile::tempdir;

const ORIGINAL_JSON: &[u8] = br#"{"value":"original"}"#;
const MUTATED_JSON: &[u8] = br#"{"value":"mutated!"}"#;
const FILE_START_OFFSET: u64 = 0;
const READ_ONLY_FILE_MODE: u32 = 0o400;

#[test]
fn read_rejects_equal_length_rewrite_even_when_bytes_are_restored() {
    assert_eq!(ORIGINAL_JSON.len(), MUTATED_JSON.len());
    let root = private_root();
    let path = root.path().join("state.json");
    write_private(&path, ORIGINAL_JSON);

    let result = durable_json::read_with_hook::<serde_json::Value, _>(&path, |stage| {
        if stage == durable_json::DurableReadStage::AfterFirstRead {
            let file = OpenOptions::new().write(true).open(&path)?;
            file.write_all_at(MUTATED_JSON, FILE_START_OFFSET)?;
            file.sync_data()?;
            file.write_all_at(ORIGINAL_JSON, FILE_START_OFFSET)?;
            file.sync_data()?;
        }
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(path).unwrap(), ORIGINAL_JSON);
}

#[test]
fn read_identity_rejects_restored_mode_after_metadata_capture() {
    let root = private_root();
    let path = root.path().join("state.json");
    write_private(&path, ORIGINAL_JSON);
    let entry = private_file::PrivateFileEntry::open(&path).unwrap();
    let opened = entry.open_read(MAX_DURABLE_JSON_BYTES).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    assert!(
        opened
            .identity
            .verify_bounded(
                &opened.file.metadata().unwrap(),
                ORIGINAL_JSON.len() as u64,
                &path,
                MAX_DURABLE_JSON_BYTES
            )
            .is_err()
    );
}

fn private_root() -> tempfile::TempDir {
    let root = tempdir().unwrap();
    create_private_dir(root.path()).unwrap();
    root
}

fn write_private(path: &Path, bytes: &[u8]) {
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    file.write_all_at(bytes, FILE_START_OFFSET).unwrap();
    file.sync_data().unwrap();
}
