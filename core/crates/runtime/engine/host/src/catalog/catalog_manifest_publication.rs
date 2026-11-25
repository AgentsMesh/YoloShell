pub(crate) const CATALOG_MANIFEST_IDENTITY_CHANGED: &str =
    "durable manifest changed before catalog publication";
pub(crate) const CATALOG_MANIFEST_VALUE_CHANGED: &str =
    "durable manifest no longer matches catalog publication";

impl Host {
    pub(crate) async fn catalog_upsert_expected_manifest(
        &self,
        manifest: &TerminalManifest,
        path: PathBuf,
        expected_identity: crate::paths::identity::FileIdentity,
    ) -> Result<()> {
        #[cfg(test)]
        if std::mem::take(&mut *lock_recover(&self.catalog_test_probes.upsert_failure)) {
            bail!(CATALOG_UPSERT_TEST_FAILURE);
        }
        let manifest = manifest.clone();
        let expected_value = serde_json::to_vec(&manifest)?;
        let namespace = Arc::clone(&self.namespace);
        let generation = self.controller_generation;
        #[cfg(test)]
        let publication_commit =
            lock_recover(&self.catalog_test_probes.manifest_publication_commit).take();
        self.catalog_operation(move |catalog| {
            require_catalog_write(catalog.upsert_for_controller_with_commit_fence(
                generation,
                &manifest,
                move || {
                    #[cfg(test)]
                    if let Some(probe) = publication_commit {
                        probe()?;
                    }
                    let (current, identity) =
                        namespace.read_json_with_identity::<TerminalManifest>(&path)?;
                    anyhow::ensure!(
                        identity == expected_identity,
                        CATALOG_MANIFEST_IDENTITY_CHANGED
                    );
                    anyhow::ensure!(
                        serde_json::to_vec(&current)? == expected_value,
                        CATALOG_MANIFEST_VALUE_CHANGED
                    );
                    Ok(())
                },
            )?)
        })
        .await
    }
}
