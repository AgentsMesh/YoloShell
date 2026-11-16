use super::super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const ORIGINAL: &[u8] = br#"{"value":"original"}"#;
const REPLACEMENT: &[u8] = br#"{"value":"replacement"}"#;

#[test]
fn atomic_write_rejects_targets_appearing_or_changing_before_rename() {
    for initially_exists in [false, true] {
        let root = private_root();
        let path = root.path().join("state.json");
        let saved = root.path().join("saved.json");
        if initially_exists {
            atomic_write_json(&path, &serde_json::json!({"value": "original"})).unwrap();
        }
        let result =
            durable_json::write_with_hook(&path, &serde_json::json!({"value": "new"}), |stage| {
                if stage == durable_json::DurableWriteStage::TemporarySynced {
                    if initially_exists {
                        fs::rename(&path, &saved)?;
                    }
                    write_private(&path, REPLACEMENT);
                }
                Ok(())
            });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), REPLACEMENT);
        assert!(!has_temporary(root.path()));
    }
}

#[test]
fn atomic_write_boundary_source_swap_restores_target_and_preserves_replacement() {
    let root = private_root();
    let path = root.path().join("state.json");
    let trusted = root.path().join("trusted-new.json");
    atomic_write_json(&path, &serde_json::json!({"value": "original"})).unwrap();

    let result =
        durable_json::write_with_boundary_hook(&path, &serde_json::json!({"value": "new"}), || {
            let temporary = temporary_path(root.path());
            fs::rename(&temporary, &trusted)?;
            write_private(&temporary, REPLACEMENT);
            Ok(())
        });

    assert!(result.is_err());
    let current: serde_json::Value = read_json(&path).unwrap();
    assert_eq!(current["value"], "original");
    assert_eq!(fs::read(temporary_path(root.path())).unwrap(), REPLACEMENT);
    let retained: serde_json::Value = read_json(&trusted).unwrap();
    assert_eq!(retained["value"], "new");
}

#[test]
fn read_rejects_target_replacement_after_first_read() {
    let root = private_root();
    let path = root.path().join("state.json");
    let saved = root.path().join("saved.json");
    write_private(&path, ORIGINAL);

    let result = durable_json::read_with_hook::<serde_json::Value, _>(&path, |stage| {
        if stage == durable_json::DurableReadStage::AfterFirstRead {
            fs::rename(&path, &saved)?;
            write_private(&path, REPLACEMENT);
        }
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT);
}

fn private_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
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

fn temporary_path(path: &Path) -> PathBuf {
    fs::read_dir(path)
        .unwrap()
        .flatten()
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(ATOMIC_FILE_PREFIX)
        })
        .unwrap()
        .path()
}
