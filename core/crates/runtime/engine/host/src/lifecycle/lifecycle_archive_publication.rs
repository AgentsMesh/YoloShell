impl Host {
    pub(crate) async fn archive_bound_session_with_fence(
        &self,
        terminal_id: Uuid,
        selection: Option<&TerminalLifecycleSelection>,
        retained_fence: Option<&LifecycleMutationFence>,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        // The catalog is derived and may lag or omit a durable recovery write.
        // Archive starts from the durable manifest so it cannot revert it.
        let existing = self
            .load_durable_manifest_optional(terminal_id)
            .map_err(internal)?
            .ok_or_else(|| ErrorBody::not_found(SESSION_NOT_FOUND))?;
        if existing.archived {
            let lifecycle_lock = self
                .acquire_archive_publication_lock(terminal_id, selection, retained_fence)
                .await?;
            let guard = self.archive_attachment_guard(terminal_id)?;
            let target = self.remember_archive_cleanup(&guard)?;
            let archived = self.clear_archived_attachments(&guard, target)?
                .ok_or_else(|| ErrorBody::conflict(LIFECYCLE_SELECTION_CHANGED))?;
            drop(guard);
            self.catalog_upsert(&archived).await.map_err(internal)?;
            drop(lifecycle_lock);
            return Ok(self.view_for_manifest(archived).await);
        }
        let runtime = match self.live_runtime_record(terminal_id).await {
            Ok(record) => Some(record),
            Err(error) => self.persisted_archive_runtime(terminal_id, error)?,
        };
        if let Some(record) = runtime.as_ref() {
            let lifecycle_lock = match (retained_fence, selection) {
                (Some(fence), _) => {
                    fence.runtime.validate_observed(Some(record))?;
                    self.acquire_retained_lifecycle_fence(terminal_id, fence)
                        .await?
                }
                (None, Some(selection)) => {
                    let (lifecycle_lock, fence) =
                        self.acquire_lifecycle_mutation_fence(selection).await?;
                    fence.runtime.validate_observed(Some(record))?;
                    lifecycle_lock
                }
                (None, None) => {
                    let lifecycle_lock = self.wait_for_resume_lifecycle_lock(terminal_id).await?;
                    self.require_no_pending_lifecycle_runtime(
                        terminal_id,
                        &LifecycleRuntimeFence::Unconditional,
                    )?;
                    lifecycle_lock
                }
            };
            self.require_archive_runtime_unchanged(terminal_id, Some(record))?;
            drop(lifecycle_lock);
            self.settle_archive_runtime(record, ARCHIVE_SETTLEMENT_WAIT)
                .await?;
        }
        let lifecycle_lock = self
            .acquire_archive_publication_lock(terminal_id, selection, retained_fence)
            .await?;
        self.require_archive_runtime_unchanged(terminal_id, runtime.as_ref())?;
        #[cfg(test)]
        if let Some(hook) = lock_recover(&self.catalog_test_probes.archive_publication).take() {
            hook();
        }
        let guard = self.archive_attachment_guard(terminal_id)?;
        guard.require_no_pending_intent().map_err(internal)?;
        let path = self.paths.manifest(terminal_id);
        let (mut manifest, identity) = guard
            .read_optional_with_identity(&path)
            .map_err(internal)?
            .ok_or_else(|| ErrorBody::not_found(SESSION_NOT_FOUND))?;
        let expected_digest = TerminalManifestDigest::capture(&existing).map_err(internal)?;
        if TerminalManifestDigest::capture(&manifest).map_err(internal)? != expected_digest {
            return Err(ErrorBody::conflict(LIFECYCLE_SELECTION_CHANGED));
        }
        manifest.archived = true;
        manifest.updated_at_ms = now_ms();
        // The manifest may already be installed if its commit fence returns an error.
        let target = self.remember_archive_cleanup(&guard)?;
        guard
            .write_json_expected_with_commit_fence(&path, Some(identity), &manifest)
            .map_err(internal)?;
        self.clear_archived_attachments(&guard, target)?
            .ok_or_else(|| ErrorBody::conflict(LIFECYCLE_SELECTION_CHANGED))?;
        drop(guard);
        self.catalog_upsert(&manifest).await.map_err(internal)?;
        drop(lifecycle_lock);
        Ok(self.view_for_manifest(manifest).await)
    }

    pub(crate) fn require_archive_runtime_unchanged(
        &self,
        terminal_id: Uuid,
        observed: Option<&RuntimeRecord>,
    ) -> std::result::Result<(), ErrorBody> {
        let path = self.paths.runtime_record(terminal_id);
        match self.read_namespace_json::<RuntimeRecord>(&path) {
            Ok(current)
                if observed.is_some_and(|observed| current.has_same_process_identity(observed)) =>
            {
                Ok(())
            }
            Err(_) if observed.is_none() && lifecycle_path_is_absent(&path) => Ok(()),
            Ok(_) | Err(_) => Err(ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED)),
        }
    }

    pub(crate) fn require_archived_runtime_settled(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<Option<RuntimeRecord>, ErrorBody> {
        let path = self.paths.runtime_record(terminal_id);
        let record = match self.read_namespace_json::<RuntimeRecord>(&path) {
            Ok(record) if runtime_loss::runtime_record_identity_is_verifiable(&record, terminal_id) => record,
            Err(_) if lifecycle_path_is_absent(&path) => return Ok(None),
            Ok(_) | Err(_) => return Err(ErrorBody::conflict(ARCHIVED_RUNTIME_CONFLICT)),
        };
        if runtime_loss::process_identities_proven_gone(&record, &self.boot_id) {
            Ok(Some(record))
        } else {
            Err(ErrorBody::conflict(ARCHIVED_RUNTIME_CONFLICT))
        }
    }

    pub(crate) async fn acquire_archive_publication_lock(
        &self,
        terminal_id: Uuid,
        selection: Option<&TerminalLifecycleSelection>,
        retained_fence: Option<&LifecycleMutationFence>,
    ) -> std::result::Result<crate::paths::private_file_lock::PrivateFileLock, ErrorBody> {
        if let Some(fence) = retained_fence {
            return self
                .acquire_retained_lifecycle_fence(terminal_id, fence)
                .await;
        }
        match selection {
            Some(selection) => self
                .acquire_lifecycle_mutation_fence(selection)
                .await
                .map(|(lifecycle_lock, _)| lifecycle_lock),
            None => {
                let lifecycle_lock = self.wait_for_resume_lifecycle_lock(terminal_id).await?;
                self.require_no_pending_lifecycle_runtime(
                    terminal_id,
                    &LifecycleRuntimeFence::Unconditional,
                )?;
                Ok(lifecycle_lock)
            }
        }
    }
}
