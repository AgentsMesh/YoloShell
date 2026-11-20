use super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

pub(super) fn prepared_temporary(
    entry: &private_file::PrivateFileEntry,
    target: &Path,
) -> private_file::TemporaryPrivateFile {
    let mut temporary = entry.temporary(TEMPORARY_PREFIX, TEMPORARY_SUFFIX).unwrap();
    temporary.file.write_all(NEW).unwrap();
    temporary.sync_and_inspect(target, u64::MAX).unwrap();
    temporary
}

pub(super) fn private_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    create_private_dir(root.path()).unwrap();
    root
}

pub(super) fn write_private(path: &Path, bytes: &[u8]) {
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

pub(super) fn staging_path(directory: &Path) -> PathBuf {
    matching_paths(directory, TEMPORARY_PREFIX).pop().unwrap()
}

pub(super) fn staging_slot_path(directory: &Path) -> PathBuf {
    matching_paths(directory, constants::PRIVATE_FILE_STAGING_PREFIX)
        .pop()
        .unwrap()
}

pub(super) fn quarantine_paths(directory: &Path) -> Vec<PathBuf> {
    matching_paths(directory, constants::PRIVATE_FILE_QUARANTINE_PREFIX)
}

pub(super) fn matching_paths(root_directory: &Path, prefix: &str) -> Vec<PathBuf> {
    fs::read_dir(root_directory)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .map(|entry| entry.path())
        .collect()
}

pub(super) fn random_residue_name() -> String {
    format!(
        "{TEMPORARY_PREFIX}{}{TEMPORARY_SUFFIX}",
        uuid::Uuid::new_v4()
    )
}
