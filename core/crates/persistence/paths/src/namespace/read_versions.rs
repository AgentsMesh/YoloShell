impl NamespaceLease {
    /// JSON, strict live CAS token and durable version observed through one FD.
    pub fn read_json_with_versions<T: DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<(
        T,
        crate::paths::identity::FileIdentity,
        crate::paths::identity::DurableFileVersion,
    )> {
        let entry = self.file(path)?;
        let value = crate::paths::read_entry_guarded_with_versions(entry, || self.verify())?;
        self.verify()?;
        Ok(value)
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/namespace_durable_versions.rs"]
mod durable_versions_tests;
