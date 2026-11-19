use super::*;
use std::ffi::CStr;

impl PrivateFileEntry {
    pub(in crate::paths) fn quarantine_named_identity(
        &self,
        source: &CStr,
        quarantine: &CStr,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        self.move_named_identity(source, quarantine, expected, max_bytes)
    }

    pub fn move_to_sibling_if_unchanged(
        &self,
        suffix: &str,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<bool> {
        let Some(current) = self.inspect(max_bytes)? else {
            return Ok(false);
        };
        anyhow::ensure!(
            current.unchanged(expected),
            "private file changed before move"
        );
        let target = self.sibling_with_suffix(suffix)?;
        anyhow::ensure!(target.is_missing()?, "private file sibling is occupied");
        self.move_named_identity(self.file_name(), target.file_name(), expected, max_bytes)?;
        Ok(true)
    }
}
