use super::FileIdentity;
use anyhow::{Result, ensure};
use std::fs::File;
use std::path::Path;

impl FileIdentity {
    pub fn inspect_file_bounded(file: &File, path: &Path, max_bytes: u64) -> Result<Self> {
        super::file_identity_descriptor::inspect(file)?.file_identity(path, max_bytes)
    }

    pub fn verify_file_bounded(
        &self,
        file: &File,
        length: u64,
        path: &Path,
        max_bytes: u64,
    ) -> Result<()> {
        let current = Self::inspect_file_bounded(file, path, max_bytes)?;
        ensure!(
            current.unchanged(*self) && current.length() == length,
            crate::ReadVersionChanged::state(path)
        );
        Ok(())
    }
}
