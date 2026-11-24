impl SessionDurableMutationGuard<'_> {
    pub fn read_optional_with_versions<T: DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<Option<(T, FileIdentity, crate::paths::identity::DurableFileVersion)>> {
        self.require_managed_path(path)?;
        if self.namespace.file_is_missing(path)? {
            return Ok(None);
        }
        self.namespace.read_json_with_versions(path).map(Some)
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/durable/durable_mutation_versioned_reads.rs"]
mod versioned_read_tests;
