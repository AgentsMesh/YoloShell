use super::*;

impl Host {
    pub(super) async fn completed_attachment_view(
        &self,
        manifest: &TerminalManifest,
        observation: Option<&RuntimeObservation>,
    ) -> Option<crate::model::CompletedAttachmentView> {
        if manifest.archived {
            return None;
        }
        let observation = observation?;
        let runtime = &observation.runtime;
        let runtime_exit = exact_natural_completion(runtime)?;
        let record = self.live_runtime_record(manifest.terminal_id).await.ok()?;
        if !runtime_view_matches(runtime, &record) {
            return None;
        }
        let authenticated = self
            .observed_completed_runtime_exit(&record, observation)
            .await?;
        if authenticated != *runtime_exit {
            return None;
        }
        Some(crate::model::CompletedAttachmentView {
            runtime_epoch: runtime.runtime_epoch,
        })
    }

    pub(super) async fn observed_completed_runtime_exit(
        &self,
        record: &RuntimeRecord,
        observation: &RuntimeObservation,
    ) -> Option<ExitRecord> {
        if self.released_natural_attachment_evidence(record) {
            return self.validate_released_natural_attachment(record).await.ok();
        }
        if !observation.authenticates(record) || !self.rpc_custody_ready(record) {
            return None;
        }
        self.validate_published_attachment(record).await.ok()?;
        exact_natural_completion(&observation.runtime).cloned()
    }

    pub(super) async fn authenticated_completed_runtime_exit(
        &self,
        record: &RuntimeRecord,
    ) -> Option<ExitRecord> {
        if self.released_natural_attachment_evidence(record) {
            return self.validate_released_natural_attachment(record).await.ok();
        }
        if !self.rpc_custody_ready(record) {
            return None;
        }
        let current = self.worker_status_if_custodied(record).await.ok()?;
        let exit = exact_natural_completion(&current)?.clone();
        self.validate_published_attachment(record).await.ok()?;
        Some(exit)
    }

    pub(super) async fn require_completed_runtime_exit(
        &self,
        record: &RuntimeRecord,
        expected: &ExitRecord,
    ) -> std::result::Result<(), ErrorBody> {
        if self
            .authenticated_completed_runtime_exit(record)
            .await
            .as_ref()
            == Some(expected)
        {
            return Ok(());
        }
        Err(ErrorBody::new(
            crate::protocol::error_code::IDENTITY_MISMATCH,
            COMPLETED_RUNTIME_AUTHORITY_CHANGED,
            false,
        ))
    }
}

pub(super) fn exact_natural_completion(runtime: &RuntimeView) -> Option<&ExitRecord> {
    let exit = runtime.exit.as_ref()?;
    (runtime.status == TerminalRuntimeStatus::Exited
        && runtime.runtime_epoch == exit.runtime_epoch
        && runtime.last_output_sequence == exit.last_output_sequence
        && exit.reason == ExitReason::Natural)
        .then_some(exit)
}
