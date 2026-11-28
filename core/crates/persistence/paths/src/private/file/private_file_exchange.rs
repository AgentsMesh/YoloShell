use super::private_file_io::exchange_at;
use super::*;

const PARENT_MISMATCH: &str = "private exchange files do not share a parent";
const SLOT_MISMATCH: &str = "private exchange file identity changed";

impl PrivateFileEntry {
    pub fn exchange_sibling_exact<F>(
        &self,
        sibling: &Self,
        current: FileIdentity,
        staged: FileIdentity,
        max_bytes: u64,
        mut commit_fence: F,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        self.require_same_parent(sibling)?;
        self.verify_exchange_before(sibling, current, staged, max_bytes)?;
        commit_fence()?;
        self.verify_exchange_before(sibling, current, staged, max_bytes)?;
        exchange_at(self.directory.descriptor(), &sibling.name, &self.name)
            .context("exchange private transaction files")?;
        self.directory.sync()?;
        self.verify_exchange_after(sibling, current, staged, max_bytes)
    }

    pub fn remove_if_exact(&self, expected: FileIdentity, max_bytes: u64) -> Result<bool> {
        self.remove_if_exact_with_fence(expected, max_bytes, || Ok(()))
    }

    pub fn remove_if_exact_with_fence<F>(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
        mut fence: F,
    ) -> Result<bool>
    where
        F: FnMut() -> Result<()>,
    {
        fence()?;
        let Some(current) = self.inspect(max_bytes)? else {
            if !self.unlink_isolation_matches(self.file_name(), expected, max_bytes)? {
                return Ok(false);
            }
            self.unlink_named_identity(self.file_name(), expected, max_bytes, || fence())?;
            fence()?;
            return Ok(true);
        };
        anyhow::ensure!(current == expected, SLOT_MISMATCH);
        self.unlink_named_identity(self.file_name(), current, max_bytes, || fence())?;
        fence()?;
        Ok(true)
    }

    pub fn remove_if_content_version(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<bool> {
        let Some(current) = self.inspect(max_bytes)? else {
            if !self.unlink_isolation_matches(self.file_name(), expected, max_bytes)? {
                return Ok(false);
            }
            self.unlink_named_identity(self.file_name(), expected, max_bytes, || Ok(()))?;
            return Ok(true);
        };
        anyhow::ensure!(current.same_content_version(expected), SLOT_MISMATCH);
        self.unlink_named_identity(self.file_name(), current, max_bytes, || Ok(()))?;
        Ok(true)
    }

    pub fn move_to_sibling_exact_with_fence<F>(
        &self,
        sibling: &Self,
        expected: FileIdentity,
        max_bytes: u64,
        mut commit_fence: F,
    ) -> Result<FileIdentity>
    where
        F: FnMut() -> Result<()>,
    {
        self.require_same_parent(sibling)?;
        self.verify_move_before(sibling, expected, max_bytes)?;
        commit_fence()?;
        self.verify_move_before(sibling, expected, max_bytes)?;
        self.move_named_identity(&self.name, &sibling.name, expected, max_bytes)?;
        let installed = sibling.open_read(max_bytes)?;
        anyhow::ensure!(
            installed.identity.same_content_version(expected),
            SLOT_MISMATCH
        );
        Ok(installed.identity)
    }

    pub fn verify_exact_identity(&self, expected: FileIdentity, max_bytes: u64) -> Result<()> {
        let current = self.open_read(max_bytes)?;
        anyhow::ensure!(current.identity == expected, SLOT_MISMATCH);
        Ok(())
    }

    fn require_same_parent(&self, sibling: &Self) -> Result<()> {
        anyhow::ensure!(
            self.name != sibling.name
                && self.path.parent() == sibling.path.parent()
                && self.directory.object_identity()? == sibling.directory.object_identity()?,
            PARENT_MISMATCH
        );
        Ok(())
    }

    fn verify_exchange_before(
        &self,
        sibling: &Self,
        current: FileIdentity,
        staged: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        self.verify_current_unchanged(current, max_bytes)?;
        sibling.verify_current_unchanged(staged, max_bytes)
    }

    fn verify_move_before(
        &self,
        sibling: &Self,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        self.verify_current_unchanged(expected, max_bytes)?;
        anyhow::ensure!(sibling.is_missing()?, SLOT_MISMATCH);
        Ok(())
    }

    fn verify_exchange_after(
        &self,
        sibling: &Self,
        current: FileIdentity,
        staged: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        // RENAME_EXCHANGE updates ctime on both inodes; object and content
        // identity remain stable and are the durable post-commit proof.
        self.verify_current_content_version(staged, max_bytes)?;
        sibling.verify_current_content_version(current, max_bytes)
    }
}
