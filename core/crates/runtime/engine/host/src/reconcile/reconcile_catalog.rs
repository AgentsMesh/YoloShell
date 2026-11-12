#[path = "recovery_manifest_publication.rs"]
pub(crate) mod recovery_manifest_publication;

impl Host {
    pub(crate) async fn persist_recovery_state(
        &self,
        manifest: &mut TerminalManifest,
        state: RecoveryState,
    ) -> bool {
        let Some(installed) = self.persist_recovery_candidate(manifest, state) else {
            return false;
        };
        match self
            .catalog_upsert_expected_manifest(
                manifest,
                self.paths.manifest(manifest.terminal_id),
                installed,
            )
            .await
        {
            Ok(()) => {
                #[cfg(test)]
                self.notify_recovery_publication(manifest);
            }
            Err(error) => {
                tracing::error!(terminal_id = %manifest.terminal_id, %error, "persist catalog recovery state");
            }
        }
        true
    }

    pub(crate) fn persist_recovery_state_durable(
        &self,
        manifest: &mut TerminalManifest,
        state: RecoveryState,
    ) -> bool {
        self.persist_recovery_candidate(manifest, state).is_some()
    }

    pub(crate) fn persist_recovery_candidate(
        &self,
        manifest: &mut TerminalManifest,
        state: RecoveryState,
    ) -> Option<crate::paths::identity::FileIdentity> {
        let original_digest = TerminalManifestDigest::capture(manifest).ok();
        let Some(candidate) = self.prepare_recovery_state(manifest, state) else {
            self.record_pending_recovery_write(manifest, state);
            return None;
        };
        let installed = match self.persist_recovery_manifest(manifest, &candidate) {
            Ok(installed) => installed,
            Err(error) => {
                self.record_pending_recovery_write(manifest, state);
                tracing::error!(terminal_id = %manifest.terminal_id, %error, "persist session recovery state");
                return None;
            }
        };
        match &original_digest {
            Some(digest) => {
                self.clear_pending_recovery_write_for_digest(manifest.terminal_id, digest)
            }
            None => self.clear_pending_recovery_write(manifest.terminal_id),
        }
        *manifest = candidate;
        Some(installed)
    }

    pub(crate) fn prepare_recovery_state(
        &self,
        manifest: &TerminalManifest,
        state: RecoveryState,
    ) -> Option<TerminalManifest> {
        if let Err(error) = self.verify_namespace() {
            tracing::error!(terminal_id = %manifest.terminal_id, %error, "skip recovery write after namespace loss");
            return None;
        }
        let mut candidate = manifest.clone();
        candidate.recovery_state = state;
        if manifest.recovery_state != state
            || !self.recovery_manifest_matches(&candidate).unwrap_or(false)
        {
            candidate.updated_at_ms = now_ms();
        }
        Some(candidate)
    }

    pub(crate) fn persist_recovery_manifest(
        &self,
        original: &TerminalManifest,
        candidate: &TerminalManifest,
    ) -> Result<crate::paths::identity::FileIdentity> {
        recovery_manifest_publication::PreparedRecoveryManifest::capture(self, original, candidate)?
            .commit(self)
    }

    pub(crate) fn recovery_manifest_matches(&self, manifest: &TerminalManifest) -> Result<bool> {
        let path = self.paths.manifest(manifest.terminal_id);
        let expected = crate::model::TerminalManifestDigest::capture(manifest)?;
        Ok(self
            .read_namespace_json::<TerminalManifest>(&path)
            .is_ok_and(|current| {
                crate::model::TerminalManifestDigest::capture(&current)
                    .is_ok_and(|actual| actual == expected)
            }))
    }

    pub(crate) async fn reconciliation_manifests(&self) -> (Vec<TerminalManifest>, usize) {
        let catalog = Arc::clone(&self.catalog);
        let namespace = Arc::clone(&self.namespace);
        let paths = self.paths.clone();
        let generation = self.controller_generation;
        #[cfg(test)]
        let probe = lock_recover(&self.catalog_test_probes.scan).take();
        let listing = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            let mut probe = probe;
            #[cfg(test)]
            if let Some(probe) = probe.as_mut() {
                probe.scan_started();
            }
            reconciliation_manifests_blocking(
                &paths,
                namespace,
                catalog,
                generation,
                #[cfg(test)]
                &mut probe,
            )
        })
        .await;
        match listing {
            Ok(listing) => listing,
            Err(error) => {
                tracing::error!(%error, "catalog recovery scan task failed");
                (Vec::new(), 1)
            }
        }
    }
}

include!("recovery_state_required.rs");
include!("reconcile_catalog_scan.rs");
include!("reconcile_catalog_repair.rs");

#[cfg(test)]
include!("../../test_support/recovery_publication_probes.rs");
