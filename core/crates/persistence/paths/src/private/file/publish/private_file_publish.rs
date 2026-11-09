use super::*;

impl PrivateFileEntry {
    pub fn rename_temporary(&self, temporary: &TemporaryPrivateFile, max_bytes: u64) -> Result<()> {
        self.rename_temporary_with_hooks(temporary, max_bytes, || Ok(()), || {})
    }

    #[cfg(test)]
    pub(crate) fn rename_temporary_with_hook<F>(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
        before_commit: F,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        self.rename_temporary_with_hooks(temporary, max_bytes, before_commit, || {})
    }

    pub(crate) fn rename_temporary_with_hooks<F, G>(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
        before_validation: F,
        after_commit: G,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
        G: FnMut(),
    {
        self.rename_temporary_with_fences(
            temporary,
            max_bytes,
            before_validation,
            || Ok(()),
            after_commit,
        )
    }

    pub fn rename_temporary_with_fences<F, G, H>(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
        mut before_validation: F,
        mut commit_fence: G,
        mut after_commit: H,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
        G: FnMut() -> Result<()>,
        H: FnMut(),
    {
        self.verify_temporary_descriptor(temporary, max_bytes)?;
        temporary.verify_digest_after_move(&self.path, max_bytes)?;
        self.verify_previous_descriptor(temporary, max_bytes)?;
        temporary.verify_previous_digest_after_move(&self.path, max_bytes)?;
        let expected = temporary.previous.as_ref().map(|file| file.identity);
        self.verify_expected(expected, max_bytes)?;
        self.verify_staging_name(temporary, max_bytes)?;
        before_validation()?;
        self.verify_temporary_descriptor(temporary, max_bytes)?;
        temporary.verify_digest_after_move(&self.path, max_bytes)?;
        self.verify_previous_descriptor(temporary, max_bytes)?;
        temporary.verify_previous_digest_after_move(&self.path, max_bytes)?;
        self.verify_expected(expected, max_bytes)?;
        self.verify_staging_name(temporary, max_bytes)?;
        match expected {
            Some(previous) => self.exchange_temporary(
                temporary,
                previous,
                max_bytes,
                &mut commit_fence,
                &mut after_commit,
            ),
            None => {
                self.install_temporary(temporary, max_bytes, &mut commit_fence, &mut after_commit)
            }
        }
    }

    fn exchange_temporary<F, G>(
        &self,
        temporary: &TemporaryPrivateFile,
        previous: FileIdentity,
        max_bytes: u64,
        commit_fence: &mut F,
        after_commit: &mut G,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
        G: FnMut(),
    {
        commit_fence()?;
        exchange_at(self.directory.descriptor(), &temporary.name, &self.name)
            .context("exchange private staging file")?;
        after_commit();
        let validation = self.verify_exchange(temporary, previous, max_bytes);
        if let Err(error) = validation {
            return self.rollback_exchange(temporary, previous, max_bytes, error, || {});
        }
        self.directory.sync()?;
        self.verify_exchange(temporary, previous, max_bytes)?;
        anyhow::ensure!(
            entry_is_missing(&self.directory, &temporary.slot)?,
            "private staging slot is already occupied"
        );
        self.move_named_identity(&temporary.name, &temporary.slot, previous, max_bytes)
    }

    pub(super) fn rollback_exchange<G>(
        &self,
        temporary: &TemporaryPrivateFile,
        previous: FileIdentity,
        max_bytes: u64,
        cause: anyhow::Error,
        mut after_rollback: G,
    ) -> Result<()>
    where
        G: FnMut(),
    {
        if let Err(proof) = self.verify_exchange(temporary, previous, max_bytes) {
            return Err(cause.context(format!(
                "private staging rollback skipped because both exchanged files could not be proven: {proof:#}"
            )));
        }
        exchange_at(self.directory.descriptor(), &temporary.name, &self.name)
            .context("roll back private staging exchange")?;
        self.directory.sync()?;
        after_rollback();
        let restored = self.verify_restored_exchange(temporary, max_bytes);
        match restored {
            Ok(()) => Err(cause.context("private staging exchange was rolled back")),
            Err(error) => Err(cause.context(format!(
                "private staging rollback retained both names but target was already replaced: {error:#}"
            ))),
        }
    }

    fn install_temporary<F, G>(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
        commit_fence: &mut F,
        after_commit: &mut G,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
        G: FnMut(),
    {
        commit_fence()?;
        rename_exclusive_at(self.directory.descriptor(), &temporary.name, &self.name)
            .context("install private staging file")?;
        after_commit();
        let validation = self.verify_installed_temporary(temporary, max_bytes);
        if let Err(error) = validation {
            let restored =
                rename_exclusive_at(self.directory.descriptor(), &self.name, &temporary.name);
            self.directory.sync()?;
            return match restored {
                Ok(()) => Err(error.context("private staging install was rolled back")),
                Err(restore) => Err(error.context(format!(
                    "private staging install failed and replacement was retained: {restore}"
                ))),
            };
        }
        self.directory.sync()?;
        self.verify_installed_temporary(temporary, max_bytes)
    }
}
