pub(crate) const RELEASED_ATTACHMENT_EVIDENCE_DETAIL: &str =
    "released PTY runtime lacks exact natural-exit attachment evidence";
pub(crate) const RELEASED_WORKER_SHUTDOWN_EVIDENCE_DETAIL: &str =
    "released PTY runtime lacks exact terminal Worker shutdown evidence";

#[derive(Clone, Copy)]
pub(crate) enum AttachmentAuthority {
    ActiveCustody,
    ReleasedNaturalExit,
}

impl Host {
    pub(crate) async fn attachment_worker_stream(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<(crate::worker::WorkerConnection, AttachmentAuthority), ErrorBody> {
        let runtime = crate::pty_custody::CustodyRuntimeIdentity::from_record(record);
        if self
            .secondary_custody
            .released_for_read_only_attachment(&runtime)
        {
            self.validate_released_natural_attachment(record).await?;
            let stream = authenticated_stream(record, self.controller_generation)
                .await
                .map_err(WorkerRpcError::into_error_body)?;
            return Ok((stream, AttachmentAuthority::ReleasedNaturalExit));
        }
        let stream = self
            .custodied_worker_stream(record)
            .await
            .map_err(WorkerRpcError::into_error_body)?;
        Ok((stream, AttachmentAuthority::ActiveCustody))
    }

    pub(crate) async fn validate_attachment_authority(
        &self,
        selected: &RuntimeRecord,
        authority: AttachmentAuthority,
    ) -> std::result::Result<(), ErrorBody> {
        match authority {
            AttachmentAuthority::ActiveCustody => {
                self.validate_published_attachment(selected).await
            }
            AttachmentAuthority::ReleasedNaturalExit => {
                self.validate_released_natural_attachment(selected)
                    .await
                    .map(|_| ())
            }
        }
    }

    pub(crate) async fn validate_released_natural_attachment(
        &self,
        selected: &RuntimeRecord,
    ) -> std::result::Result<ExitRecord, ErrorBody> {
        let current = self.live_runtime_record(selected.terminal_id).await?;
        if !attachment_runtime_matches(&current, selected)
            || current.controller_generation != self.controller_generation
        {
            return Err(released_worker_evidence_error(
                RELEASED_ATTACHMENT_EVIDENCE_DETAIL,
            ));
        }
        self.released_natural_attachment_exit(selected)
            .ok_or_else(|| released_worker_evidence_error(RELEASED_ATTACHMENT_EVIDENCE_DETAIL))
    }

    pub(crate) fn validate_released_terminal_worker_shutdown(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<(), ErrorBody> {
        if self.released_terminal_worker_shutdown_evidence(record) {
            return Ok(());
        }
        Err(released_worker_evidence_error(
            RELEASED_WORKER_SHUTDOWN_EVIDENCE_DETAIL,
        ))
    }

    pub(crate) fn released_natural_attachment_evidence(&self, record: &RuntimeRecord) -> bool {
        self.released_natural_attachment_exit(record).is_some()
    }

    pub(crate) fn released_natural_attachment_exit(&self, record: &RuntimeRecord) -> Option<ExitRecord> {
        let runtime = crate::pty_custody::CustodyRuntimeIdentity::from_record(record);
        self.secondary_custody
            .released_for_read_only_attachment(&runtime)
            .then(|| self.exact_released_exit(record))
            .flatten()
            .filter(|exit| exit.reason == ExitReason::Natural)
    }

    pub(crate) fn released_terminal_worker_shutdown_evidence(&self, record: &RuntimeRecord) -> bool {
        let runtime = crate::pty_custody::CustodyRuntimeIdentity::from_record(record);
        self.secondary_custody
            .released_for_terminal_worker_shutdown(&runtime)
            && self.exact_released_exit(record).is_some()
    }

    pub(crate) fn exact_released_exit(&self, record: &RuntimeRecord) -> Option<ExitRecord> {
        let current = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(record.terminal_id));
        let exit =
            self.read_namespace_json::<ExitRecord>(&self.paths.exit_record(record.terminal_id));
        if process_identity_is_gone(record.child_pid, record.child_start_time)
            && matches!(current, Ok(current) if current.has_same_process_identity(record)
                && current.controller_generation == self.controller_generation)
        {
            return exit
                .ok()
                .filter(|exit| exit.runtime_epoch == record.runtime_epoch);
        }
        None
    }
}

pub(crate) fn released_worker_evidence_error(detail: &'static str) -> ErrorBody {
    ErrorBody::new(
        crate::protocol::error_code::IDENTITY_MISMATCH,
        detail,
        false,
    )
}
