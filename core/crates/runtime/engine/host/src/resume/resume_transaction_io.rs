use super::*;

pub(crate) const RESUME_TRANSACTION_FAILURE_LOG: &str =
    "durable resume transaction failed validation";

impl Host {
    pub(crate) fn read_optional_resume_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<Option<T>> {
        if self.namespace.file_is_missing(path)? {
            Ok(None)
        } else {
            self.read_namespace_json(path).map(Some)
        }
    }

    pub(crate) fn load_resume_transaction(
        &self,
        terminal_id: Uuid,
    ) -> Result<Option<ResumeTransaction>> {
        let path = self.paths.resume_transaction(terminal_id);
        if self.namespace.file_is_missing(&path)? {
            return Ok(None);
        }
        let transaction = self.read_namespace_json::<ResumeTransaction>(&path)?;
        transaction.validate_for(terminal_id)?;
        Ok(Some(transaction))
    }

    pub(crate) fn publish_resume_transaction(&self, transaction: &ResumeTransaction) -> Result<()> {
        transaction.validate_for(transaction.terminal_id)?;
        self.write_namespace_json(
            &self.paths.resume_transaction(transaction.terminal_id),
            transaction,
        )?;
        self.forget_parked_resume_retry(transaction.terminal_id);
        Ok(())
    }

    pub(crate) fn remove_resume_transaction(&self, terminal_id: Uuid) -> Result<()> {
        self.namespace
            .remove_file(&self.paths.resume_transaction(terminal_id))?;
        self.forget_parked_resume_retry(terminal_id);
        Ok(())
    }

    pub(crate) fn load_durable_manifest_optional(
        &self,
        terminal_id: Uuid,
    ) -> Result<Option<TerminalManifest>> {
        let path = self.paths.manifest(terminal_id);
        match self.namespace.file_is_missing(&path) {
            Ok(true) => return Ok(None),
            Ok(false) => {}
            Err(error) if error_is_not_found(&error) => return Ok(None),
            Err(error) => return Err(error),
        }
        self.load_durable_manifest(terminal_id).map(Some)
    }

    pub(crate) fn load_durable_manifest(&self, terminal_id: Uuid) -> Result<TerminalManifest> {
        let manifest =
            self.read_namespace_json::<TerminalManifest>(&self.paths.manifest(terminal_id))?;
        anyhow::ensure!(
            manifest.schema_version == SESSION_MANIFEST_SCHEMA_VERSION
                && manifest.terminal_id == terminal_id,
            RESUME_MANIFEST_MISMATCH
        );
        Ok(manifest)
    }

    pub(crate) async fn persist_resume_manifest(
        &self,
        manifest: &TerminalManifest,
    ) -> std::result::Result<(), SessionResumeFailure> {
        self.write_namespace_json(&self.paths.manifest(manifest.terminal_id), manifest)
            .map_err(internal)?;
        self.catalog_upsert(manifest).await.map_err(internal)?;
        Ok(())
    }

    pub(crate) async fn persist_resume_failure_catalog(&self, terminal_id: Uuid) {
        let Ok(manifest) = self.load_durable_manifest(terminal_id) else {
            return;
        };
        if manifest.recovery_state != RecoveryState::Uncertain {
            return;
        }
        if let Err(error) = self.catalog_upsert(&manifest).await {
            tracing::error!(%terminal_id, %error, "persist failed resume recovery state in catalog");
        }
    }

    pub(crate) fn resume_transaction_failure(
        &self,
        manifest: &mut TerminalManifest,
        message: impl std::fmt::Display,
    ) -> SessionResumeFailure {
        let failure = self.resume_transaction_failure_body(manifest.terminal_id, message);
        self.persist_recovery_state_durable(manifest, RecoveryState::Uncertain);
        failure
    }

    pub(crate) fn resume_transaction_failure_body(
        &self,
        terminal_id: Uuid,
        message: impl std::fmt::Display,
    ) -> SessionResumeFailure {
        let diagnostic = format!("{message:#}");
        tracing::warn!(
            %terminal_id,
            error = %diagnostic,
            RESUME_TRANSACTION_FAILURE_LOG
        );
        ErrorBody::new(
            crate::protocol::error_code::CONFLICT,
            format!("{RESUME_TRANSACTION_INVALID}: {message}"),
            false,
        )
        .into()
    }
}

pub(crate) fn error_is_not_found(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
    })
}
