impl NamespaceLease {
    pub fn read_json_with_identity<T: DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<(T, crate::paths::identity::FileIdentity)> {
        let entry = self.file(path)?;
        let value = crate::paths::read_entry_guarded_with_identity(entry, || self.verify())?;
        self.verify()?;
        Ok(value)
    }

    pub fn write_json_expected_with_commit_fence<T, F>(
        &self,
        path: &Path,
        expected: Option<crate::paths::identity::FileIdentity>,
        value: &T,
        mut commit_fence: F,
    ) -> Result<crate::paths::identity::FileIdentity>
    where
        T: Serialize,
        F: FnMut() -> Result<()>,
    {
        let entry = self.file(path)?;
        let verification = entry.reopen()?;
        let identity = crate::paths::write_entry_expected_with_commit_fence(
            entry,
            expected,
            value,
            || self.verify(),
            || {
                self.verify()?;
                commit_fence()
            },
        )?;
        self.verify()?;
        verification
            .verify_exact_identity(identity, crate::paths::constants::MAX_DURABLE_JSON_BYTES)?;
        self.verify()?;
        Ok(identity)
    }

    pub fn write_json_expected_with_commit_observer<T, F, G>(
        &self,
        path: &Path,
        expected: Option<crate::paths::identity::FileIdentity>,
        value: &T,
        mut commit_fence: F,
        after_commit: G,
    ) -> Result<crate::paths::identity::FileIdentity>
    where
        T: Serialize,
        F: FnMut() -> Result<()>,
        G: FnMut(crate::paths::identity::FileIdentity),
    {
        let entry = self.file(path)?;
        let verification = entry.reopen()?;
        let identity = crate::paths::write_entry_expected_with_commit_observer(
            entry,
            expected,
            value,
            || self.verify(),
            || {
                self.verify()?;
                commit_fence()
            },
            after_commit,
        )?;
        self.verify()?;
        verification
            .verify_exact_identity(identity, crate::paths::constants::MAX_DURABLE_JSON_BYTES)?;
        self.verify()?;
        Ok(identity)
    }

    pub(crate) fn write_json_expected_with_commit_fence_with_target_lock_authority<T, F>(
        &self,
        path: &Path,
        expected: Option<crate::paths::identity::FileIdentity>,
        value: &T,
        authority: &crate::paths::private_file::TargetLockAuthority<'_>,
        mut commit_fence: F,
    ) -> Result<crate::paths::identity::FileIdentity>
    where
        T: Serialize,
        F: FnMut() -> Result<()>,
    {
        let entry = self.file(path)?;
        let verification = entry.reopen()?;
        let identity =
            crate::paths::write_entry_expected_with_commit_fence_with_target_lock_authority(
                entry,
                expected,
                value,
                authority,
                || self.verify(),
                || {
                    self.verify()?;
                    commit_fence()
                },
            )?;
        self.verify()?;
        verification
            .verify_exact_identity(identity, crate::paths::constants::MAX_DURABLE_JSON_BYTES)?;
        self.verify()?;
        Ok(identity)
    }

    pub(crate) fn write_json_expected_with_commit_observer_with_target_lock_authority<T, F, G>(
        &self,
        path: &Path,
        expected: Option<crate::paths::identity::FileIdentity>,
        value: &T,
        authority: &crate::paths::private_file::TargetLockAuthority<'_>,
        mut commit_fence: F,
        after_commit: G,
    ) -> Result<crate::paths::identity::FileIdentity>
    where
        T: Serialize,
        F: FnMut() -> Result<()>,
        G: FnMut(crate::paths::identity::FileIdentity),
    {
        let entry = self.file(path)?;
        let verification = entry.reopen()?;
        let identity =
            crate::paths::write_entry_expected_with_commit_observer_with_target_lock_authority(
                entry,
                expected,
                value,
                authority,
                || self.verify(),
                || {
                    self.verify()?;
                    commit_fence()
                },
                after_commit,
            )?;
        self.verify()?;
        verification
            .verify_exact_identity(identity, crate::paths::constants::MAX_DURABLE_JSON_BYTES)?;
        self.verify()?;
        Ok(identity)
    }

    pub fn write_json_with_commit_fence<T, F>(
        &self,
        path: &Path,
        value: &T,
        mut commit_fence: F,
    ) -> Result<()>
    where
        T: Serialize,
        F: FnMut() -> Result<()>,
    {
        let entry = self.file(path)?;
        crate::paths::atomic_write_entry_guarded_with_commit_fence(
            entry,
            value,
            || self.verify(),
            || {
                self.verify()?;
                commit_fence()
            },
        )?;
        self.verify()
    }

    pub fn ensure_directory_with_commit_fence<F>(
        &self,
        path: &Path,
        mut commit_fence: F,
    ) -> Result<PrivateDirectory>
    where
        F: FnMut() -> Result<()>,
    {
        let anchor = self.anchor(path)?;
        let directory = anchor
            .directory
            .ensure_descendant_with_commit_fence(path, || {
                self.verify()?;
                commit_fence()
            })?;
        self.verify()?;
        Ok(directory)
    }
}
