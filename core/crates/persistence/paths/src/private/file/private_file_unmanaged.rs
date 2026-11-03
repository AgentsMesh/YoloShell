use super::PrivateFileEntry;
use super::private_file_io::entry_stat;
use anyhow::{Context, Result, ensure};

const DIRECTORY_ENTRY_ERROR: &str = "unmanaged private entry is a directory";
const IDENTITY_CHANGED_ERROR: &str = "unmanaged private entry changed before unlink";
const UNLINK_CONTEXT: &str = "unlink unmanaged private entry";

impl PrivateFileEntry {
    pub fn unlink_unmanaged_deferred(&self) -> Result<bool> {
        self.directory.verify_path()?;
        let Some(first) = inspect(self)? else {
            return Ok(false);
        };
        ensure!(
            first.st_mode & libc::S_IFMT != libc::S_IFDIR,
            DIRECTORY_ENTRY_ERROR
        );
        self.directory.verify_path()?;
        let second = inspect(self)?.context(IDENTITY_CHANGED_ERROR)?;
        ensure!(unchanged(&first, &second), IDENTITY_CHANGED_ERROR);
        let result = unsafe { libc::unlinkat(self.directory.descriptor(), self.name.as_ptr(), 0) };
        if result != 0 {
            return Err(std::io::Error::last_os_error()).context(UNLINK_CONTEXT);
        }
        ensure!(inspect(self)?.is_none(), IDENTITY_CHANGED_ERROR);
        self.directory.verify_path()?;
        Ok(true)
    }

    pub(in crate::paths) fn unlink_exact_deferred(
        &self,
        expected: crate::paths::identity::FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        self.directory.verify_path()?;
        ensure!(
            self.inspect(max_bytes)?
                .is_some_and(|value| value == expected),
            IDENTITY_CHANGED_ERROR
        );
        self.directory.verify_path()?;
        ensure!(
            self.inspect(max_bytes)?
                .is_some_and(|value| value == expected),
            IDENTITY_CHANGED_ERROR
        );
        let result = unsafe { libc::unlinkat(self.directory.descriptor(), self.name.as_ptr(), 0) };
        if result != 0 {
            return Err(std::io::Error::last_os_error()).context(UNLINK_CONTEXT);
        }
        ensure!(self.inspect(max_bytes)?.is_none(), IDENTITY_CHANGED_ERROR);
        self.directory.verify_path()
    }
}

fn inspect(entry: &PrivateFileEntry) -> Result<Option<libc::stat>> {
    match entry_stat(entry.directory.descriptor(), &entry.name) {
        Ok(stat) => Ok(Some(stat)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn unchanged(left: &libc::stat, right: &libc::stat) -> bool {
    left.st_dev == right.st_dev
        && left.st_ino == right.st_ino
        && left.st_mode == right.st_mode
        && left.st_nlink == right.st_nlink
        && left.st_uid == right.st_uid
        && left.st_gid == right.st_gid
        && left.st_size == right.st_size
        && left.st_mtime == right.st_mtime
        && left.st_mtime_nsec == right.st_mtime_nsec
        && left.st_ctime == right.st_ctime
        && left.st_ctime_nsec == right.st_ctime_nsec
}
