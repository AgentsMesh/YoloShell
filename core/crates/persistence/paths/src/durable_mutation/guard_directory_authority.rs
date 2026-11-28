impl SessionDurableMutationGuard<'_> {
    pub(crate) fn serialized_write_authority(
        &self,
        path: &Path,
    ) -> Result<crate::paths::private_file::ParentLockAuthority<'_>> {
        self.require_managed_path(path)?;
        self.namespace
            .file(path)?
            .parent_lock_authority(&self._directory_lock)
    }
}
