use super::snapshot::{
    SnapshotReadStage, SnapshotWriteStage, read_snapshot_with_test_hook,
    write_snapshot_with_test_hook,
};
use super::tests_security_support::*;
use super::*;
use std::fs;
use std::os::unix::fs::symlink;

#[test]
fn snapshot_detects_target_replacement_before_install() {
    let root = private_root();
    let path = root.path().join("snapshot");
    let saved = root.path().join("saved");
    let initial = sample_snapshot(ORIGINAL_BYTES);
    write_snapshot(&path, &initial).unwrap();
    let result = write_snapshot_with_test_hook(&path, &sample_snapshot(b"new"), |stage| {
        if stage == SnapshotWriteStage::TemporarySynced {
            replace_target(&path, &saved);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_BYTES);
    let restored = read_snapshot(&saved).unwrap();
    assert_eq!(restored.runtime_epoch, initial.runtime_epoch);
    assert_eq!(restored.data, initial.data);
}

#[test]
fn snapshot_detects_target_replacement_after_install() {
    let root = private_root();
    let path = root.path().join("snapshot");
    let saved = root.path().join("saved");
    write_snapshot(&path, &sample_snapshot(ORIGINAL_BYTES)).unwrap();
    let result = write_snapshot_with_test_hook(&path, &sample_snapshot(b"new"), |stage| {
        if stage == SnapshotWriteStage::Installed {
            replace_target(&path, &saved);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_BYTES);
    assert_eq!(read_snapshot(&saved).unwrap().data, b"new");
}

#[test]
fn snapshot_detects_parent_replacement_before_install() {
    let root = private_root();
    let parent = root.path().join("terminal");
    set_private_directory(&parent);
    let path = parent.join("snapshot");
    write_snapshot(&path, &sample_snapshot(ORIGINAL_BYTES)).unwrap();
    let mut moved_snapshot = None;
    let result = write_snapshot_with_test_hook(&path, &sample_snapshot(b"new"), |stage| {
        if stage == SnapshotWriteStage::TemporarySynced {
            let (moved, _) = move_parent(&path);
            moved_snapshot = Some(moved);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_BYTES);
    assert_eq!(
        read_snapshot(moved_snapshot.as_ref().unwrap())
            .unwrap()
            .data,
        ORIGINAL_BYTES
    );
}

#[test]
fn snapshot_read_detects_path_and_content_changes() {
    let root = private_root();
    let path = root.path().join("snapshot");
    let saved = root.path().join("saved");
    write_snapshot(&path, &sample_snapshot(ORIGINAL_BYTES)).unwrap();
    let replaced = read_snapshot_with_test_hook(&path, |stage| {
        assert_eq!(stage, SnapshotReadStage::AfterFirstRead);
        replace_target(&path, &saved);
        Ok(())
    });
    assert!(replaced.is_err());
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_BYTES);

    fs::rename(&saved, &path).unwrap();
    let changed = read_snapshot_with_test_hook(&path, |_| {
        let mut bytes = fs::read(&path)?;
        let last = bytes.last_mut().unwrap();
        *last ^= u8::MAX;
        fs::write(&path, bytes)?;
        Ok(())
    });
    assert!(changed.is_err());
}

#[test]
fn snapshot_never_follows_a_replaced_temporary_name() {
    let root = private_root();
    let path = root.path().join("snapshot");
    let outside = root.path().join("outside");
    write_private(&outside, ORIGINAL_BYTES);
    let result = write_snapshot_with_test_hook(&path, &sample_snapshot(b"new"), |stage| {
        if stage == SnapshotWriteStage::TemporarySynced {
            let temporary = temporary_path(root.path(), SNAPSHOT_TEMPORARY_PREFIX);
            fs::remove_file(&temporary)?;
            symlink(&outside, temporary)?;
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&outside).unwrap(), ORIGINAL_BYTES);
    assert!(!path.exists());
}
