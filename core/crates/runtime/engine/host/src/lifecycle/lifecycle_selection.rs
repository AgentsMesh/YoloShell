pub(crate) const LIFECYCLE_SELECTION_CHANGED: &str =
    "session lifecycle selection no longer matches current state";
pub(crate) const LIFECYCLE_ACTIVE_SESSION: &str =
    "conditional discard cannot terminate an active session";
pub(crate) const LIFECYCLE_RUNTIME_EPOCH_CHANGED: &str =
    "session runtime epoch no longer matches the lifecycle request";

impl Host {
    pub(crate) fn validate_runtime_epoch_fence(
        &self,
        selection: &TerminalLifecycleSelection,
    ) -> std::result::Result<(), ErrorBody> {
        self.capture_runtime_epoch_fence(selection).map(|_| ())
    }

    pub(crate) fn capture_runtime_epoch_fence(
        &self,
        selection: &TerminalLifecycleSelection,
    ) -> std::result::Result<Option<RuntimeRecord>, ErrorBody> {
        if selection.require_runtime_absent {
            if selection.expected_runtime_epoch.is_some()
                || !lifecycle_path_is_absent(&self.paths.runtime_record(selection.terminal_id))
            {
                return Err(ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED));
            }
            return Ok(None);
        }
        let Some(expected) = selection.expected_runtime_epoch else {
            return Ok(None);
        };
        let current = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(selection.terminal_id))
            .map_err(|_| ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED))?;
        if expected.is_nil()
            || current.schema_version != RUNTIME_RECORD_SCHEMA_VERSION
            || current.terminal_id != selection.terminal_id
            || current.runtime_epoch.is_nil()
            || current.runtime_epoch != expected
        {
            return Err(ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED));
        }
        Ok(Some(current))
    }

    pub(crate) async fn validate_stop_selection(
        &self,
        selection: &TerminalLifecycleSelection,
    ) -> std::result::Result<(), ErrorBody> {
        match self.validate_product_selection(selection).await {
            Ok(()) => Ok(()),
            Err(error) if self.selection_already_stopped(selection).await? => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn selection_already_stopped(
        &self,
        selection: &TerminalLifecycleSelection,
    ) -> std::result::Result<bool, ErrorBody> {
        let Some(expected) = selection.expected_status else {
            return Ok(false);
        };
        if !matches!(
            expected,
            TerminalRuntimeStatus::Starting
                | TerminalRuntimeStatus::Running
                | TerminalRuntimeStatus::Finalizing
                | TerminalRuntimeStatus::OutputBlocked
                | TerminalRuntimeStatus::Lost
                | TerminalRuntimeStatus::Unreachable
        ) {
            return Ok(false);
        }
        let manifest = self
            .catalog_get(selection.terminal_id)
            .await
            .map_err(internal)?
            .ok_or_else(|| ErrorBody::not_found(SESSION_NOT_FOUND))?;
        if manifest.archived {
            return Ok(false);
        }
        Ok(self
            .runtime_view_for_manifest(&manifest)
            .await
            .is_some_and(|runtime| runtime.status == TerminalRuntimeStatus::Exited))
    }

    pub(crate) async fn validate_product_selection(
        &self,
        selection: &TerminalLifecycleSelection,
    ) -> std::result::Result<(), ErrorBody> {
        let manifest = self
            .catalog_get(selection.terminal_id)
            .await
            .map_err(internal)?
            .ok_or_else(|| ErrorBody::not_found(SESSION_NOT_FOUND))?;
        if manifest_digest_changed(selection, &manifest) {
            return Err(ErrorBody::conflict(LIFECYCLE_SELECTION_CHANGED));
        }
        if selection
            .expected_updated_at_ms
            .is_some_and(|expected| expected != manifest.updated_at_ms)
        {
            return Err(ErrorBody::conflict(LIFECYCLE_SELECTION_CHANGED));
        }
        let Some(expected_status) = selection.expected_status else {
            return Ok(());
        };
        if manifest.archived {
            // Archive state is not a runtime status; a selection that expects
            // a concrete runtime can no longer match this session.
            return Err(ErrorBody::conflict(LIFECYCLE_SELECTION_CHANGED));
        }
        let current_status = self
            .runtime_view_for_manifest(&manifest)
            .await
            .map(|runtime| runtime.status)
            .unwrap_or(TerminalRuntimeStatus::Unreachable);
        self.validate_lifecycle_catalog_fence(selection).await?;
        if current_status != expected_status {
            return Err(ErrorBody::conflict(LIFECYCLE_SELECTION_CHANGED));
        }
        Ok(())
    }

    pub(crate) async fn revalidate_lifecycle_fence(
        &self,
        selection: &TerminalLifecycleSelection,
    ) -> std::result::Result<LifecycleMutationFence, ErrorBody> {
        let runtime = self.capture_lifecycle_mutation_runtime(selection)?;
        self.validate_lifecycle_catalog_fence(selection).await?;
        #[cfg(test)]
        if let Some(hook) = lock_recover(&self.catalog_test_probes.lifecycle_revalidation).take() {
            hook();
        }
        Ok(LifecycleMutationFence {
            runtime,
            selection: selection.clone(),
        })
    }

    pub(crate) async fn validate_lifecycle_catalog_fence(
        &self,
        selection: &TerminalLifecycleSelection,
    ) -> std::result::Result<(), ErrorBody> {
        if selection.expected_updated_at_ms.is_some()
            || selection.expected_manifest_sha256.is_some()
        {
            let manifest = self
                .catalog_get(selection.terminal_id)
                .await
                .map_err(internal)?
                .ok_or_else(|| ErrorBody::not_found(SESSION_NOT_FOUND))?;
            let updated_at_changed = selection
                .expected_updated_at_ms
                .is_some_and(|expected| manifest.updated_at_ms != expected);
            if updated_at_changed || manifest_digest_changed(selection, &manifest) {
                return Err(ErrorBody::conflict(LIFECYCLE_SELECTION_CHANGED));
            }
        }
        Ok(())
    }
}

pub(crate) fn manifest_digest_changed(
    selection: &TerminalLifecycleSelection,
    manifest: &TerminalManifest,
) -> bool {
    selection
        .expected_manifest_sha256
        .as_ref()
        .is_some_and(|expected| {
            crate::model::TerminalManifestDigest::capture(manifest)
                .map(|actual| actual != *expected)
                .unwrap_or(true)
        })
}
