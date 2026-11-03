use super::*;

impl PrivateFileEntry {
    pub fn is_missing(&self) -> Result<bool> {
        retained_entry_is_missing(&self.directory, &self.name)
    }

    pub fn inspect(&self, max_bytes: u64) -> Result<Option<FileIdentity>> {
        if entry_is_missing(&self.directory, &self.name)? {
            return Ok(None);
        }
        self.open_read(max_bytes)
            .map(|opened| Some(opened.identity))
    }

    pub fn verify_current_unchanged(&self, expected: FileIdentity, max_bytes: u64) -> Result<()> {
        let current = self.open_once(read_flags(), max_bytes)?;
        anyhow::ensure!(
            current.identity.unchanged(expected),
            crate::ReadVersionChanged::private_file()
        );
        self.directory.verify_path()
    }

    pub fn verify_current_content_version(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        let current = self.open_once(read_flags(), max_bytes)?;
        anyhow::ensure!(
            current.identity.same_content_version(expected),
            "private file content version changed"
        );
        self.directory.verify_path()
    }

    pub fn verify_expected(&self, expected: Option<FileIdentity>, max_bytes: u64) -> Result<()> {
        let current = self.inspect(max_bytes)?;
        let unchanged = match (expected, current) {
            (Some(expected), Some(current)) => current.unchanged(expected),
            (None, None) => true,
            _ => false,
        };
        anyhow::ensure!(
            unchanged,
            "private file changed before replacement: {}",
            self.path.display()
        );
        self.directory.verify_path()
    }

    pub(crate) fn verify_published_content_version(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        let current = inspect_named(
            self.directory.descriptor(),
            &self.name,
            &self.path,
            max_bytes,
        )?;
        anyhow::ensure!(
            current.same_content_version(expected),
            "private file path changed"
        );
        self.directory.verify_descriptor_path()
    }

    pub fn sync_parent(&self) -> Result<()> {
        self.directory.verify_path()?;
        self.directory.sync()?;
        self.directory.verify_path()
    }

    pub(crate) fn verify_retained_identity(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        let current = inspect_named(
            self.directory.descriptor(),
            &self.name,
            &self.path,
            max_bytes,
        )?;
        anyhow::ensure!(current.unchanged(expected), "private file version changed");
        Ok(())
    }
}
