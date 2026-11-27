use crate::ReadVersionChanged;
use crate::paths::{self, constants::*, durable_json, private_file::PrivateFileEntry};
use std::os::unix::fs::PermissionsExt;

const FILE_NAME: &str = "read-version.json";
const LINK_NAME: &str = "read-version-link.json";
const ORIGINAL: &[u8] = br#"{"value":1}"#;
const UPDATED: &[u8] = br#"{"value":22}"#;
const CORRUPT: &[u8] = b"not-json";
const UNREADABLE_MODE: u32 = 0o000;
const READ_CONTEXT: &str = "read runtime observation";
const STATE_CHANGED: &str = "state changed while reading";
const PRIVATE_CHANGED: &str = "private file version changed";

#[test]
fn double_read_change_is_typed_without_changing_its_diagnostic() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(FILE_NAME);
    paths::atomic_write_json(&path, &serde_json::json!({"value": 1})).unwrap();
    let error = durable_json::read_with_hook::<serde_json::Value, _>(&path, |stage| {
        if stage == durable_json::DurableReadStage::AfterFirstRead {
            std::fs::write(&path, UPDATED)?;
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("{STATE_CHANGED}: {}", path.display())
    );
    assert!(
        error
            .context(READ_CONTEXT)
            .downcast_ref::<ReadVersionChanged>()
            .is_some()
    );
}

#[test]
fn descriptor_metadata_and_named_version_conflicts_are_typed() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(FILE_NAME);
    paths::atomic_write_json(&path, &serde_json::json!({"value": 1})).unwrap();
    let entry = PrivateFileEntry::open(&path).unwrap();
    let opened = entry.open_read(MAX_DURABLE_JSON_BYTES).unwrap();
    let original = opened.identity();
    std::fs::write(&path, UPDATED).unwrap();
    for error in [
        original
            .verify_file_bounded(
                opened.file(),
                ORIGINAL.len() as u64,
                &path,
                MAX_DURABLE_JSON_BYTES,
            )
            .unwrap_err(),
        original
            .verify_bounded(
                &opened.file().metadata().unwrap(),
                ORIGINAL.len() as u64,
                &path,
                MAX_DURABLE_JSON_BYTES,
            )
            .unwrap_err(),
    ] {
        assert_eq!(
            error.to_string(),
            format!("{STATE_CHANGED}: {}", path.display())
        );
        assert!(error.downcast_ref::<ReadVersionChanged>().is_some());
    }
    let updated: serde_json::Value = serde_json::from_slice(UPDATED).unwrap();
    paths::atomic_write_json(&path, &updated).unwrap();
    let error = entry
        .verify_current_unchanged(original, MAX_DURABLE_JSON_BYTES)
        .unwrap_err();
    assert_eq!(error.to_string(), PRIVATE_CHANGED);
    assert!(error.downcast_ref::<ReadVersionChanged>().is_some());
}

#[test]
fn missing_corrupt_permissions_and_hard_links_are_not_read_version_conflicts() {
    for scenario in [
        Failure::Missing,
        Failure::Corrupt,
        Failure::Permissions,
        Failure::HardLink,
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(FILE_NAME);
        paths::atomic_write_json(&path, &serde_json::json!({"value": 1})).unwrap();
        match scenario {
            Failure::Missing => std::fs::remove_file(&path).unwrap(),
            Failure::Corrupt => std::fs::write(&path, CORRUPT).unwrap(),
            Failure::Permissions => {
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(UNREADABLE_MODE))
                    .unwrap()
            }
            Failure::HardLink => std::fs::hard_link(&path, root.path().join(LINK_NAME)).unwrap(),
        }
        let error = paths::read_json::<serde_json::Value>(&path).unwrap_err();
        assert!(
            error.downcast_ref::<ReadVersionChanged>().is_none(),
            "{error:#}"
        );
    }
}

enum Failure {
    Missing,
    Corrupt,
    Permissions,
    HardLink,
}
