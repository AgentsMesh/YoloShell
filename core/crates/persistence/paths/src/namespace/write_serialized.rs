impl NamespaceLease {
    pub(crate) fn write_json_expected_serialized<T, F, G>(
        &self,
        path: &Path,
        expected: Option<crate::paths::identity::FileIdentity>,
        value: &T,
        authority: &crate::paths::private_file::ParentLockAuthority<'_>,
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
        let identity = crate::paths::write_entry_expected_serialized(
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
}
