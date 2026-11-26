use super::super::*;
use std::cell::Cell;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink};
use tempfile::tempdir;

const ORIGINAL_JSON: &[u8] = br#"{"value":"original"}"#;
const REPLACEMENT_JSON: &[u8] = br#"{"value":"replacement"}"#;
const UNSAFE_FILE_MODE: u32 = 0o644;

#[test]
fn atomic_write_rejects_symlink_and_hardlink_targets() {
    let root = private_root();
    let outside = root.path().join("outside.json");
    write_private(&outside, ORIGINAL_JSON);
    let symlink_path = root.path().join("symlink.json");
    symlink(&outside, &symlink_path).unwrap();
    assert!(atomic_write_json(&symlink_path, &serde_json::json!({"new": true})).is_err());
    assert_eq!(fs::read(&outside).unwrap(), ORIGINAL_JSON);
    assert!(
        fs::symlink_metadata(&symlink_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    let hardlink_path = root.path().join("hardlink.json");
    fs::hard_link(&outside, &hardlink_path).unwrap();
    assert!(atomic_write_json(&hardlink_path, &serde_json::json!({"new": true})).is_err());
    assert_eq!(fs::read(&outside).unwrap(), ORIGINAL_JSON);
}

#[test]
fn read_rejects_symlink_hardlink_unsafe_mode_and_fifo() {
    let root = private_root();
    let outside = root.path().join("outside.json");
    write_private(&outside, ORIGINAL_JSON);
    let symlink_path = root.path().join("symlink.json");
    symlink(&outside, &symlink_path).unwrap();
    assert!(read_json::<serde_json::Value>(&symlink_path).is_err());

    let hardlink_path = root.path().join("hardlink.json");
    fs::hard_link(&outside, &hardlink_path).unwrap();
    assert!(read_json::<serde_json::Value>(&hardlink_path).is_err());
    fs::remove_file(&hardlink_path).unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(UNSAFE_FILE_MODE)).unwrap();
    assert!(read_json::<serde_json::Value>(&outside).is_err());

    let fifo = root.path().join("fifo.json");
    let fifo_name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifo(fifo_name.as_ptr(), PRIVATE_FILE_MODE as libc::mode_t) },
        0
    );
    assert!(read_json::<serde_json::Value>(&fifo).is_err());
}

#[test]
fn read_rejects_growth_after_metadata_and_after_eof() {
    for stage in [
        durable_json::DurableReadStage::AfterMetadata,
        durable_json::DurableReadStage::AfterFirstRead,
    ] {
        let root = private_root();
        let path = root.path().join("state.json");
        atomic_write_json(&path, &serde_json::json!({"value": "original"})).unwrap();
        let changed = Cell::new(false);
        let result = durable_json::read_with_hook::<serde_json::Value, _>(&path, |current| {
            if current == stage && !changed.replace(true) {
                OpenOptions::new()
                    .append(true)
                    .open(&path)?
                    .write_all(b" ")?;
            }
            Ok(())
        });
        assert!(result.is_err());
        assert!(changed.get());
    }
}

#[test]
fn atomic_write_detects_parent_and_installed_target_swaps() {
    let root = private_root();
    let parent = root.path().join("parent");
    create_private_dir(&parent).unwrap();
    let path = parent.join("state.json");
    let moved = root.path().join("moved");
    let swapped = Cell::new(false);
    let result = durable_json::write_with_hook(&path, &serde_json::json!({"new": true}), |stage| {
        if stage == durable_json::DurableWriteStage::TemporarySynced && !swapped.replace(true) {
            fs::rename(&parent, &moved)?;
            create_private_dir(&parent)?;
            write_private(&path, REPLACEMENT_JSON);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_JSON);
    assert!(!has_temporary(&moved));

    let target = root.path().join("target.json");
    let saved = root.path().join("saved.json");
    let result =
        durable_json::write_with_hook(&target, &serde_json::json!({"new": true}), |stage| {
            if stage == durable_json::DurableWriteStage::Installed {
                fs::rename(&target, &saved)?;
                write_private(&target, REPLACEMENT_JSON);
            }
            Ok(())
        });
    assert!(result.is_err());
    assert_eq!(fs::read(&target).unwrap(), REPLACEMENT_JSON);
}

#[test]
fn read_detects_parent_and_target_swaps() {
    let root = private_root();
    let parent = root.path().join("parent");
    create_private_dir(&parent).unwrap();
    let path = parent.join("state.json");
    atomic_write_json(&path, &serde_json::json!({"value": "original"})).unwrap();
    let moved = root.path().join("moved");
    let result = durable_json::read_with_hook::<serde_json::Value, _>(&path, |stage| {
        if stage == durable_json::DurableReadStage::AfterFirstRead {
            fs::rename(&parent, &moved)?;
            create_private_dir(&parent)?;
            write_private(&path, REPLACEMENT_JSON);
        }
        Ok(())
    });
    assert!(result.is_err());
}

#[test]
fn durable_json_is_bounded_and_temporary_files_are_cleaned() {
    let root = private_root();
    let path = root.path().join("oversized.json");
    write_private(
        &path,
        &vec![b' '; MAX_DURABLE_JSON_BYTES as usize + SIZE_LIMIT_SENTINEL_BYTES as usize],
    );
    assert!(read_json::<serde_json::Value>(&path).is_err());
    assert!(!has_temporary(root.path()));
}

fn private_root() -> tempfile::TempDir {
    let root = tempdir().unwrap();
    create_private_dir(root.path()).unwrap();
    root
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    file.write_all(bytes).unwrap();
}

fn has_temporary(path: &Path) -> bool {
    fs::read_dir(path).unwrap().flatten().any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .starts_with(ATOMIC_FILE_PREFIX)
    })
}
