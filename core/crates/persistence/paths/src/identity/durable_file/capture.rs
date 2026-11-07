use super::*;
use std::{fs::File, os::fd::AsFd, path::Path};

impl DurableFileVersion {
    /// Capture both evidence forms from one retained FD, fencing the entire read.
    /// The caller still owns pathname checks and any required mutation lock.
    pub fn capture_file_bounded(
        file: &File,
        path: &Path,
        max_bytes: u64,
    ) -> Result<(Self, FileIdentity)> {
        let before = FileIdentity::inspect_file_bounded(file, path, max_bytes)?;
        let object = DurableObjectIdentity::capture(file.as_fd())?;
        let after = FileIdentity::inspect_file_bounded(file, path, max_bytes)?;
        ensure!(
            before.unchanged(after),
            crate::ReadVersionChanged::state(path)
        );
        Ok((Self::from_live(object, after)?, after))
    }

    /// Verify the complete persisted version and mint a fresh live CAS token.
    /// Keep the owner's mutation lock held through the subsequent CAS operation.
    pub fn verify_file_bounded(
        &self,
        file: &File,
        path: &Path,
        max_bytes: u64,
    ) -> Result<FileIdentity> {
        let (current, live) = Self::capture_file_bounded(file, path, max_bytes)?;
        ensure!(
            self.unchanged(current),
            crate::ReadVersionChanged::state(path)
        );
        Ok(live)
    }

    /// A proven rename may change ctime while retaining the same content version.
    pub fn verify_content_version_bounded(
        &self,
        file: &File,
        path: &Path,
        max_bytes: u64,
    ) -> Result<FileIdentity> {
        let (current, live) = Self::capture_file_bounded(file, path, max_bytes)?;
        ensure!(
            self.same_content_version(current),
            crate::ReadVersionChanged::state(path)
        );
        Ok(live)
    }
}
