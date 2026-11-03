use super::super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const STATE_NAME: &str = "matrix-state.json";
const SAVED_NAME: &str = "matrix-saved.json";
const ORIGINAL_VALUE: &str = "original";
const NEXT_VALUE: &str = "next";
const ORIGINAL_JSON: &[u8] = br#"{"value":"original"}"#;
const REPLACEMENT_JSON: &[u8] = br#"{"value":"replacement"}"#;
const ACTION_FAILURE: &str = "injected durable JSON action failure";
const FIRST_BYTE_INDEX: usize = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WriteAction {
    Install,
    Replace,
    FailTemporary,
    FailInstalled,
    TargetAppears,
    InstalledChanges,
}

#[test]
fn one_write_instance_covers_install_replace_failure_and_races() {
    for action in [
        WriteAction::Install,
        WriteAction::Replace,
        WriteAction::FailTemporary,
        WriteAction::FailInstalled,
        WriteAction::TargetAppears,
        WriteAction::InstalledChanges,
    ] {
        let root = private_root();
        let path = root.path().join(STATE_NAME);
        if action == WriteAction::Replace {
            atomic_write_json(&path, &value(ORIGINAL_VALUE)).unwrap();
        }
        let result = durable_json::write_with_hook(&path, &value(NEXT_VALUE), |stage| {
            match (action, stage) {
                (WriteAction::FailTemporary, durable_json::DurableWriteStage::TemporarySynced)
                | (WriteAction::FailInstalled, durable_json::DurableWriteStage::Installed) => {
                    anyhow::bail!(ACTION_FAILURE)
                }
                (WriteAction::TargetAppears, durable_json::DurableWriteStage::TemporarySynced) => {
                    write_private(&path, ORIGINAL_JSON);
                    Ok(())
                }
                (WriteAction::InstalledChanges, durable_json::DurableWriteStage::Installed) => {
                    fs::write(&path, REPLACEMENT_JSON)?;
                    Ok(())
                }
                _ => Ok(()),
            }
        });
        let expected = matches!(action, WriteAction::Install | WriteAction::Replace);
        assert_eq!(result.is_ok(), expected, "{action:?}: {result:?}");
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommitAction {
    Success,
    Failure,
    SourceSwap,
}

#[test]
fn one_commit_instance_covers_success_failure_and_source_replacement() {
    for action in [
        CommitAction::Success,
        CommitAction::Failure,
        CommitAction::SourceSwap,
    ] {
        let root = private_root();
        let path = root.path().join(STATE_NAME);
        let saved = root.path().join(SAVED_NAME);
        let result =
            durable_json::write_with_boundary_hook(&path, &value(NEXT_VALUE), || match action {
                CommitAction::Failure => anyhow::bail!(ACTION_FAILURE),
                CommitAction::SourceSwap => {
                    let temporary = temporary_path(root.path());
                    fs::rename(&temporary, &saved)?;
                    write_private(&temporary, REPLACEMENT_JSON);
                    Ok(())
                }
                CommitAction::Success => Ok(()),
            });
        assert_eq!(result.is_ok(), action == CommitAction::Success);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReadAction {
    Stable,
    FailMetadata,
    FailFirstRead,
    ChangeMetadata,
    ChangeFirstRead,
}

#[test]
fn one_read_instance_covers_stability_hook_failures_and_content_races() {
    for action in [
        ReadAction::Stable,
        ReadAction::FailMetadata,
        ReadAction::FailFirstRead,
        ReadAction::ChangeMetadata,
        ReadAction::ChangeFirstRead,
    ] {
        let root = private_root();
        let path = root.path().join(STATE_NAME);
        atomic_write_json(&path, &value(ORIGINAL_VALUE)).unwrap();
        let result = durable_json::read_with_hook::<serde_json::Value, _>(&path, |stage| {
            match (action, stage) {
                (ReadAction::FailMetadata, durable_json::DurableReadStage::AfterMetadata)
                | (ReadAction::FailFirstRead, durable_json::DurableReadStage::AfterFirstRead) => {
                    anyhow::bail!(ACTION_FAILURE)
                }
                (ReadAction::ChangeMetadata, durable_json::DurableReadStage::AfterMetadata)
                | (ReadAction::ChangeFirstRead, durable_json::DurableReadStage::AfterFirstRead) => {
                    let mut bytes = fs::read(&path)?;
                    bytes[FIRST_BYTE_INDEX] ^= u8::MAX;
                    fs::write(&path, bytes)?;
                    Ok(())
                }
                _ => Ok(()),
            }
        });
        assert_eq!(result.is_ok(), action == ReadAction::Stable);
    }
}

fn value(text: &str) -> serde_json::Value {
    serde_json::json!({"value": text})
}

fn private_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    create_private_dir(root.path()).unwrap();
    root
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn temporary_path(directory: &Path) -> PathBuf {
    fs::read_dir(directory)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(ATOMIC_FILE_PREFIX)
        })
        .unwrap()
}
