use super::private_file_names::retired_name;
use super::*;

const PRIVATE_FILE_CHANGED_BEFORE_REMOVE: &str = "private file changed before remove";

impl PrivateFileEntry {
    pub fn remove(&self) -> Result<()> {
        self.remove_with(None, || Ok(()))
    }

    pub fn remove_if_unchanged(&self, expected: FileIdentity) -> Result<()> {
        self.remove_with(Some(expected), || Ok(()))
    }

    pub fn remove_if_unchanged_with_fence(
        &self,
        expected: FileIdentity,
        fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        self.remove_with(Some(expected), fence)
    }

    fn remove_with<F>(&self, expected: Option<FileIdentity>, mut before_retire: F) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        if entry_is_missing(&self.directory, &self.name)? {
            anyhow::ensure!(expected.is_none(), PRIVATE_FILE_CHANGED_BEFORE_REMOVE);
            return Ok(());
        }
        let opened = self.open_stable(write_flags(), u64::MAX)?;
        anyhow::ensure!(
            expected.is_none_or(|expected| opened.identity.unchanged(expected)),
            PRIVATE_FILE_CHANGED_BEFORE_REMOVE
        );
        let _process_lock = lock_private_file(&opened.file)?;
        let retired = retired_name(&self.name)?;
        anyhow::ensure!(
            entry_is_missing(&self.directory, &retired)?,
            "retired private file slot is already occupied"
        );
        before_retire()?;
        self.move_named_identity(&self.name, &retired, opened.identity, u64::MAX)?;
        opened
            .file
            .set_len(0)
            .context("truncate retired private file")?;
        opened
            .file
            .sync_all()
            .context("sync retired private file")?;
        let current = open_named(
            self.directory.descriptor(),
            &retired,
            &self.path,
            read_flags(),
            u64::MAX,
        )?;
        anyhow::ensure!(
            current.identity.same_object(opened.identity),
            "retired file changed"
        );
        self.directory.verify_path()
    }

    #[cfg(any(test, feature = "persistence-test-support"))]
    pub(crate) fn remove_with_hook<F>(&self, hook: F) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        self.remove_with(None, hook)
    }
}
