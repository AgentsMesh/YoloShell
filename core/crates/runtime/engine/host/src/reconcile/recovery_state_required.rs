const REQUIRED_RECOVERY_PREPARATION_CONTEXT: &str = "prepare required session recovery state";

impl Host {
    pub(crate) async fn persist_recovery_state_required(
        &self,
        manifest: &mut TerminalManifest,
        state: RecoveryState,
    ) -> Result<()> {
        let requires_manifest_write = manifest.recovery_state != state;
        let candidate = if !requires_manifest_write {
            self.verify_namespace()?;
            manifest.clone()
        } else {
            self.prepare_recovery_state(manifest, state)
                .context(REQUIRED_RECOVERY_PREPARATION_CONTEXT)?
        };
        let installed = self.persist_recovery_manifest(manifest, &candidate)?;
        self.catalog_upsert_expected_manifest(
            &candidate,
            self.paths.manifest(candidate.terminal_id),
            installed,
        )
        .await
        .with_context(|| {
            recovery_manifest_publication::RecoveryCatalogFailure::new(candidate.clone(), installed)
        })?;
        *manifest = candidate;
        #[cfg(test)]
        self.notify_recovery_publication(manifest);
        Ok(())
    }
}
