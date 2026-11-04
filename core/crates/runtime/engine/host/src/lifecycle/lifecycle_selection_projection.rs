impl Host {
    pub(crate) async fn durable_product_selection(
        &self,
        mut selection: TerminalLifecycleSelection,
    ) -> std::result::Result<TerminalLifecycleSelection, ErrorBody> {
        if !selection.is_exact_catalog_selection() {
            return Ok(selection);
        }
        self.validate_runtime_epoch_fence(&selection)?;
        let manifest = self
            .catalog_get(selection.terminal_id)
            .await
            .map_err(internal)?
            .ok_or_else(|| ErrorBody::not_found(SESSION_NOT_FOUND))?;
        if !manifest_digest_changed(&selection, &manifest) {
            return Ok(selection);
        }
        let observation = self.runtime_observation_for_manifest(&manifest).await;
        let mut projected = manifest.clone();
        self.project_runtime_recovery_state(&mut projected, observation.as_ref());
        let runtime_matches = observation.as_ref().is_some_and(|observation| {
            selection.expected_runtime_epoch == Some(observation.runtime.runtime_epoch)
                && selection.expected_status == Some(observation.runtime.status)
        });
        if !runtime_matches
            || selection.expected_updated_at_ms != Some(manifest.updated_at_ms)
            || manifest_digest_changed(&selection, &projected)
        {
            return Err(ErrorBody::conflict(LIFECYCLE_SELECTION_CHANGED));
        }
        // A display-only projection cannot become the catalog mutation fence.
        selection.expected_manifest_sha256 =
            Some(crate::model::TerminalManifestDigest::capture(&manifest).map_err(internal)?);
        self.validate_runtime_epoch_fence(&selection)?;
        self.validate_lifecycle_catalog_fence(&selection).await?;
        Ok(selection)
    }
}
