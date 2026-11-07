use super::private_file_io::{open_named, read_flags, rename_exclusive_at};
use super::private_file_missing::retained_entry_is_missing;
use super::private_file_names::{retained_isolation_name, retired_name};
use super::*;

impl PrivateFileEntry {
    pub fn open_isolated_retained(&self, max_bytes: u64) -> Result<Option<OpenedPrivateFile>> {
        let name = retained_isolation_name(&self.name)?;
        match open_named(
            self.directory.descriptor(),
            &name,
            &self.path,
            read_flags(),
            max_bytes,
        ) {
            Ok(opened) => Ok(Some(opened)),
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn verify_isolated_retained(&self, expected: FileIdentity, max_bytes: u64) -> Result<()> {
        let current = self
            .open_isolated_retained(max_bytes)?
            .context("isolated retained file is missing")?;
        anyhow::ensure!(
            current.identity.same_content_version(expected),
            "isolated retained file changed"
        );
        Ok(())
    }

    pub fn restore_isolated_retained(&self, expected: FileIdentity, max_bytes: u64) -> Result<()> {
        self.restore_isolated_retained_with_hook(expected, max_bytes, || {})
    }

    pub(super) fn restore_isolated_retained_with_hook<F>(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
        mut after_move: F,
    ) -> Result<()>
    where
        F: FnMut(),
    {
        let isolated = retained_isolation_name(&self.name)?;
        self.verify_isolated_retained(expected, max_bytes)?;
        anyhow::ensure!(
            retained_entry_is_missing(&self.directory, &self.name)?,
            "retained target is occupied"
        );
        rename_exclusive_at(self.directory.descriptor(), &isolated, &self.name)
            .context("restore isolated retained file")?;
        after_move();
        if let Err(error) = self.verify_retained_content(expected, max_bytes) {
            let restored = rename_exclusive_at(self.directory.descriptor(), &self.name, &isolated);
            let _ = self.directory.sync();
            return match restored {
                Ok(()) => Err(error.context("invalid retained recovery was rolled back")),
                Err(rollback) => Err(error.context(format!(
                    "invalid retained recovery was preserved at target: {rollback}"
                ))),
            };
        }
        self.directory.sync()?;
        self.verify_retained_content(expected, max_bytes)?;
        Ok(())
    }

    pub fn recycle_isolated_retained(&self, expected: FileIdentity, max_bytes: u64) -> Result<()> {
        self.recycle_isolated_retained_with_hook(expected, max_bytes, || {})
    }

    pub(super) fn recycle_isolated_retained_with_hook<F>(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
        mut after_move: F,
    ) -> Result<()>
    where
        F: FnMut(),
    {
        let isolated = retained_isolation_name(&self.name)?;
        let retired = retired_name(&self.name)?;
        self.verify_isolated_retained(expected, max_bytes)?;
        anyhow::ensure!(
            retained_entry_is_missing(&self.directory, &retired)?,
            "retained staging slot is occupied"
        );
        rename_exclusive_at(self.directory.descriptor(), &isolated, &retired)
            .context("recycle isolated retained file")?;
        after_move();
        if let Err(error) = self.verify_named_retained(&retired, expected, max_bytes) {
            let restored = rename_exclusive_at(self.directory.descriptor(), &retired, &isolated);
            let _ = self.directory.sync();
            return match restored {
                Ok(()) => Err(error.context("invalid retained recycle was rolled back")),
                Err(rollback) => Err(error.context(format!(
                    "invalid retained recycle was preserved: {rollback}"
                ))),
            };
        }
        self.directory.sync()?;
        self.verify_named_retained(&retired, expected, max_bytes)
    }

    fn verify_named_retained(
        &self,
        name: &std::ffi::CStr,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        let opened = open_named(
            self.directory.descriptor(),
            name,
            &self.path,
            read_flags(),
            max_bytes,
        )?;
        anyhow::ensure!(
            opened.identity.same_content_version(expected),
            "retained slot changed"
        );
        Ok(())
    }
}
