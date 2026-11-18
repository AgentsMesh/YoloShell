use super::secure_vfs_file::ArtifactKind;
use anyhow::{Context, Result, ensure};
use rusqlite::ffi;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const SQLITE_JOURNAL_SUFFIX: &str = "-journal";
const SQLITE_WAL_SUFFIX: &str = "-wal";
const SQLITE_URI_PARAMETER_COUNT: libc::c_int = 0;
const FILENAME_ALLOCATION_DETAIL: &str = "SQLite filename layout allocation failed";
const FILENAME_KIND_DETAIL: &str = "SQLite SHM does not have a standalone filename layout";
const FILENAME_ENCODING_CONTEXT: &str = "encode SQLite alias filename";

pub(super) struct SqliteFilename {
    database: ffi::sqlite3_filename,
    opened: ffi::sqlite3_filename,
}

impl SqliteFilename {
    pub(super) fn create(main: &Path, kind: ArtifactKind) -> Result<Self> {
        ensure!(kind != ArtifactKind::Shm, FILENAME_KIND_DETAIL);
        let database = path_string(main)?;
        let journal = path_string(&append_suffix(main, SQLITE_JOURNAL_SUFFIX))?;
        let wal = path_string(&append_suffix(main, SQLITE_WAL_SUFFIX))?;
        let allocated = unsafe {
            ffi::sqlite3_create_filename(
                database.as_ptr(),
                journal.as_ptr(),
                wal.as_ptr(),
                SQLITE_URI_PARAMETER_COUNT,
                std::ptr::null_mut(),
            )
        };
        ensure!(!allocated.is_null(), FILENAME_ALLOCATION_DETAIL);
        let mut filename = Self {
            database: allocated,
            opened: std::ptr::null(),
        };
        filename.opened = unsafe {
            match kind {
                ArtifactKind::Main => allocated,
                ArtifactKind::Journal => ffi::sqlite3_filename_journal(allocated),
                ArtifactKind::Wal => ffi::sqlite3_filename_wal(allocated),
                ArtifactKind::Shm => unreachable!(),
            }
        };
        ensure!(!filename.opened.is_null(), FILENAME_ALLOCATION_DETAIL);
        Ok(filename)
    }

    pub(super) fn opened(&self) -> ffi::sqlite3_filename {
        self.opened
    }
}

// SQLite owns the allocation and permits a sqlite3_file to move across threads.
unsafe impl Send for SqliteFilename {}

impl Drop for SqliteFilename {
    fn drop(&mut self) {
        unsafe { ffi::sqlite3_free_filename(self.database) };
    }
}

fn path_string(path: &Path) -> Result<CString> {
    CString::new(path.as_os_str().as_bytes()).context(FILENAME_ENCODING_CONTEXT)
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/secure_vfs_filename_tests.rs"]
mod tests;
