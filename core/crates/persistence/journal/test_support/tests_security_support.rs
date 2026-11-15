use super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub(super) const ORIGINAL_BYTES: &[u8] = b"original";
pub(super) const REPLACEMENT_BYTES: &[u8] = b"replacement";
pub(super) const UNSAFE_FILE_MODE: u32 = 0o644;
const TEST_BARRIER_SEQUENCE: u64 = 7;
const TEST_TERMINAL_COLUMNS: u16 = 80;
const TEST_TERMINAL_ROWS: u16 = 24;

pub(super) fn private_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    crate::paths::ensure_private_parent(&root.path().join("entry")).unwrap();
    root
}

pub(super) fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

pub(super) fn set_private_directory(path: &Path) {
    fs::create_dir(path).unwrap();
    fs::set_permissions(
        path,
        fs::Permissions::from_mode(crate::paths::test_support::private_directory_mode()),
    )
    .unwrap();
}

pub(super) fn sample_snapshot(data: &[u8]) -> TerminalSnapshot {
    TerminalSnapshot {
        runtime_epoch: Uuid::new_v4(),
        barrier_sequence: TEST_BARRIER_SEQUENCE,
        cols: TEST_TERMINAL_COLUMNS,
        rows: TEST_TERMINAL_ROWS,
        data: data.to_vec(),
    }
}

pub(super) fn replace_target(path: &Path, saved: &Path) {
    fs::rename(path, saved).unwrap();
    write_private(path, REPLACEMENT_BYTES);
}

pub(super) fn move_parent(path: &Path) -> (PathBuf, PathBuf) {
    let parent = path.parent().unwrap();
    let moved = parent.with_extension("moved");
    fs::rename(parent, &moved).unwrap();
    set_private_directory(parent);
    let replacement = parent.join(path.file_name().unwrap());
    write_private(&replacement, REPLACEMENT_BYTES);
    (moved.join(path.file_name().unwrap()), replacement)
}

pub(super) fn temporary_path(directory: &Path, prefix: &str) -> PathBuf {
    fs::read_dir(directory)
        .unwrap()
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .unwrap()
        .path()
}

pub(super) fn stale_name(prefix: &str) -> String {
    format!("{prefix}{}{TEMPORARY_SUFFIX}", Uuid::new_v4())
}
