use super::*;
use yolo_host_archive::AttachmentTarget;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

impl Host {
    pub(super) fn archive_attachment_guard(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<SessionDurableMutationGuard<'_>, ErrorBody> {
        SessionDurableMutationGuard::acquire_after_lifecycle_lock(
            &self.paths,
            &self.namespace,
            terminal_id,
        )
        .map_err(internal)
    }

    pub(super) fn remember_archive_cleanup(
        &self,
        guard: &SessionDurableMutationGuard<'_>,
    ) -> std::result::Result<AttachmentTarget, ErrorBody> {
        let target = AttachmentTarget::capture(&self.paths, guard).map_err(internal)?;
        lock_recover(&self.pending_attachment_cleanup).insert(guard.terminal_id(), target);
        Ok(target)
    }

    pub(super) fn clear_archived_attachments(
        &self,
        guard: &SessionDurableMutationGuard<'_>,
        target: AttachmentTarget,
    ) -> std::result::Result<Option<TerminalManifest>, ErrorBody> {
        let id = guard.terminal_id();
        guard.require_no_pending_intent().map_err(internal)?;
        let archived = self
            .load_durable_manifest_optional(id)
            .map_err(internal)?
            .filter(|manifest| manifest.archived);
        if archived.is_some() {
            self.require_no_pending_lifecycle_runtime(id, &LifecycleRuntimeFence::Unconditional)?;
            let runtime = self.require_archived_runtime_settled(id)?;
            let cleared =
                yolo_host_archive::clear_staged_attachments_if(&self.paths, guard, target, || {
                    runtime.as_ref().is_none_or(|record| {
                        lifecycle_runtime_is_valid(record, id)
                            && yolo_host_archive::exit_or_tombstone_matches(
                                &self.paths,
                                &self.namespace,
                                record,
                            )
                    })
                })
                .map_err(internal)?;
            if !cleared {
                return Err(ErrorBody::conflict(ARCHIVED_RUNTIME_CONFLICT));
            }
        }
        lock_recover(&self.pending_attachment_cleanup).remove(&id);
        Ok(archived)
    }

    pub(crate) async fn retry_archived_attachment_cleanup(&self) -> usize {
        let pending = lock_recover(&self.pending_attachment_cleanup).clone();
        let mut failures = 0;
        for (id, identity) in pending {
            let _session = self.session_locks.lock(id).await;
            if lock_recover(&self.pending_attachment_cleanup).get(&id) != Some(&identity) {
                continue;
            }
            if let Err(error) = self.retry_attachment_cleanup(id, identity) {
                failures += 1;
                tracing::warn!(terminal_id = %id, ?error, "archived attachment cleanup pending");
            }
        }
        failures
    }

    fn retry_attachment_cleanup(
        &self,
        id: Uuid,
        target: AttachmentTarget,
    ) -> std::result::Result<(), ErrorBody> {
        let Some(guard) = target
            .reacquire(&self.paths, &self.namespace, id)
            .map_err(internal)?
        else {
            lock_recover(&self.pending_attachment_cleanup).remove(&id);
            return Ok(());
        };
        self.clear_archived_attachments(&guard, target).map(|_| ())
    }
}
