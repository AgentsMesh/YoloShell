use super::*;

impl PrivateFileEntry {
    pub(super) fn verify_installed_temporary(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
    ) -> Result<()> {
        self.verify_installed_temporary_with(temporary, max_bytes, || Ok(()))
    }

    fn verify_installed_temporary_with<F>(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
        after_digest: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.verify_published_content_version(temporary.identity, max_bytes)?;
        let verified = temporary.verify_digest_after_move(&self.path, max_bytes)?;
        self.verify_current_unchanged(verified, max_bytes)?;
        after_digest()?;
        self.verify_current_unchanged(verified, max_bytes)
    }

    #[cfg(test)]
    pub(crate) fn verify_installed_temporary_with_hook<F>(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
        hook: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.verify_installed_temporary_with(temporary, max_bytes, hook)
    }

    pub(super) fn verify_exchange(
        &self,
        temporary: &TemporaryPrivateFile,
        previous: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        self.verify_installed_temporary(temporary, max_bytes)?;
        let displaced = temporary
            .verify_previous_digest_after_move(&self.path, max_bytes)?
            .context("private exchange is missing previous content digest")?;
        anyhow::ensure!(
            displaced.same_content_version(previous),
            "displaced private target content version changed"
        );
        self.verify_named_unchanged(&temporary.name, displaced, max_bytes)?;
        Ok(())
    }

    pub(super) fn verify_restored_exchange(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
    ) -> Result<()> {
        let previous = temporary
            .verify_previous_digest_after_move(&self.path, max_bytes)?
            .context("private exchange is missing previous content digest")?;
        self.verify_current_unchanged(previous, max_bytes)?;
        let staged = temporary.verify_digest_after_move(&self.path, max_bytes)?;
        self.verify_named_unchanged(&temporary.name, staged, max_bytes)
    }

    pub(super) fn verify_staging_name(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
    ) -> Result<()> {
        let current = open_named(
            self.directory.descriptor(),
            &temporary.name,
            &self.path,
            read_flags(),
            max_bytes,
        )?;
        anyhow::ensure!(
            current.identity.unchanged(temporary.identity),
            "temporary file changed"
        );
        Ok(())
    }

    pub(super) fn verify_temporary_descriptor(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
    ) -> Result<()> {
        temporary.identity.verify_file_bounded(
            &temporary.file,
            temporary.identity.length(),
            &self.path,
            max_bytes,
        )
    }

    pub(super) fn verify_previous_descriptor(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
    ) -> Result<()> {
        let Some(previous) = &temporary.previous else {
            return Ok(());
        };
        previous.identity.verify_file_bounded(
            &previous.file,
            previous.identity.length(),
            &self.path,
            max_bytes,
        )
    }

    fn verify_named_unchanged(
        &self,
        name: &std::ffi::CStr,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        let current = open_named(
            self.directory.descriptor(),
            name,
            &self.path,
            read_flags(),
            max_bytes,
        )?;
        anyhow::ensure!(
            current.identity.unchanged(expected),
            "private published name changed"
        );
        Ok(())
    }
}
