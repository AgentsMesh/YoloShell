use super::super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const CYCLE_COUNT: usize = 4;
const MAX_RECYCLE_SLOTS: usize = 2;
const ORIGINAL: &[u8] = b"remove-original";
const REPLACEMENT: &[u8] = b"remove-boundary-replacement";
const TARGET_NAME: &str = "binding.json";

#[test]
fn remove_boundary_swap_restores_replacement_without_unlinking_it() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let saved = root.path().join("remove-original-saved");
    write_private(&target, ORIGINAL);
    let entry = private_file::PrivateFileEntry::open(&target).unwrap();

    let result = entry.remove_with_hook(|| {
        fs::rename(&target, &saved)?;
        write_private(&target, REPLACEMENT);
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(&target).unwrap(), REPLACEMENT);
    assert_eq!(fs::read(&saved).unwrap(), ORIGINAL);
    assert!(matching_slots(root.path(), constants::PRIVATE_FILE_RETIRED_PREFIX).is_empty());
}

#[test]
fn remove_and_rewrite_cycles_reuse_two_bounded_slots() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    for cycle in 0..CYCLE_COUNT {
        atomic_write_json(&target, &serde_json::json!({"cycle": cycle})).unwrap();
        atomic_write_json(
            &target,
            &serde_json::json!({"cycle": cycle, "updated": true}),
        )
        .unwrap();
        let entry = private_file::PrivateFileEntry::open(&target).unwrap();
        entry.remove().unwrap();
        entry.remove().unwrap();
        assert!(!target.exists());
        assert!(recycle_slot_count(root.path()) <= MAX_RECYCLE_SLOTS);
    }
}

fn recycle_slot_count(directory: &Path) -> usize {
    matching_slots(directory, constants::PRIVATE_FILE_STAGING_PREFIX).len()
        + matching_slots(directory, constants::PRIVATE_FILE_RETIRED_PREFIX).len()
}

fn matching_slots(directory: &Path, prefix: &str) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .map(|entry| entry.path())
        .collect()
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
        .mode(constants::PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(constants::PRIVATE_FILE_MODE))
        .unwrap();
    file.write_all(bytes).unwrap();
}
