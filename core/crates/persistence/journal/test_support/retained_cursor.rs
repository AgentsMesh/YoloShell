use super::constants::COMMIT_FILE_SUFFIX;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub fn retained_cursor_path_for_test(journal: &Path) -> PathBuf {
    let mut name = journal.file_name().map(OsString::from).unwrap_or_default();
    name.push(COMMIT_FILE_SUFFIX);
    journal.with_file_name(name)
}
