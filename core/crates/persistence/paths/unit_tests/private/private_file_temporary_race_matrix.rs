use super::super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const TARGET_NAME: &str = "temporary-race-target";
const STAGE_PREFIX: &str = ".temporary-race-";
const STAGE_SUFFIX: &str = ".stage";
const ORIGINAL_BYTES: &[u8] = b"temporary-race-original";
const REPLACEMENT_BYTES: &[u8] = b"temporary-race-replacement";
const EMPTY_FILE_BYTES: u64 = 0;

#[test]
fn target_disappearance_between_absence_check_and_open_retries_to_success() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL_BYTES);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let removed = target.clone();
    let _hook = entry.install_before_target_open_hook(move || {
        fs::remove_file(removed)?;
        Ok(())
    });

    let temporary = entry.temporary(STAGE_PREFIX, STAGE_SUFFIX).unwrap();

    assert!(temporary.previous.is_none());
    entry.remove_temporary(&temporary).unwrap();
}

#[test]
fn stage_replacement_after_identity_capture_retries_without_using_stale_authority() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let directory = root.path().to_path_buf();
    let _hook = entry.install_after_stage_identity_hook(move || {
        let stage = active_stage(&directory);
        fs::remove_file(&stage)?;
        write_private(&stage, REPLACEMENT_BYTES);
        Ok(())
    });

    let temporary = entry.temporary(STAGE_PREFIX, STAGE_SUFFIX).unwrap();

    assert_eq!(temporary.file.metadata().unwrap().len(), EMPTY_FILE_BYTES);
    entry.remove_temporary(&temporary).unwrap();
}

fn active_stage(directory: &Path) -> PathBuf {
    fs::read_dir(directory)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(STAGE_PREFIX)
        })
        .unwrap()
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
