pub(crate) const ARCHIVED_RUNTIME_CONFLICT: &str =
    "archived session still has potentially live runtime custody";

impl Host {
    pub(crate) async fn archive_bound_session(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        self.archive_bound_session_with_fence(terminal_id, None, None)
            .await
    }

    pub(crate) async fn archive_selected_session(
        &self,
        selection: &TerminalLifecycleSelection,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        self.archive_bound_session_with_fence(selection.terminal_id, Some(selection), None)
            .await
    }

    pub(crate) async fn archive_selected_session_with_retained_fence(
        &self,
        selection: &TerminalLifecycleSelection,
        fence: &LifecycleMutationFence,
    ) -> std::result::Result<TerminalView, ErrorBody> {
        self.archive_bound_session_with_fence(selection.terminal_id, Some(selection), Some(fence))
            .await
    }

    pub(crate) fn persisted_archive_runtime(
        &self,
        terminal_id: Uuid,
        live_error: ErrorBody,
    ) -> std::result::Result<Option<RuntimeRecord>, ErrorBody> {
        let path = self.paths.runtime_record(terminal_id);
        match self.read_namespace_json::<RuntimeRecord>(&path) {
            Ok(record) if runtime_loss::runtime_record_identity_is_verifiable(&record, terminal_id) => {
                Ok(Some(record))
            }
            Ok(_) => Err(live_error),
            Err(_) if path.try_exists().unwrap_or(true) => Err(live_error),
            Err(_) => Ok(None),
        }
    }

    pub(crate) async fn settle_archive_runtime(
        &self,
        record: &RuntimeRecord,
        evidence_timeout: Duration,
    ) -> std::result::Result<(), ErrorBody> {
        self.settle_archive_runtime_inner(record, evidence_timeout)
            .await
    }

    pub(crate) async fn settle_archive_runtime_inner(
        &self,
        record: &RuntimeRecord,
        evidence_timeout: Duration,
    ) -> std::result::Result<(), ErrorBody> {
        let custody_runtime = crate::pty_custody::CustodyRuntimeIdentity::from_record(record);
        if self
            .secondary_custody
            .released_for_terminal_worker_shutdown(&custody_runtime)
            && !process_identity_is_gone(record.worker_pid, record.worker_start_time)
        {
            return match self.retire_released_worker_classified(record).await {
                Ok(()) => {
                    self.settle_disconnected_archive(record, evidence_timeout)
                        .await
                }
                Err(error) if error.may_settle_after_disconnect() => {
                    self.settle_disconnected_archive(record, evidence_timeout)
                        .await
                }
                Err(error) => Err(error.into_error_body()),
            };
        }
        if self
            .secondary_custody
            .retired_for_settlement(&custody_runtime)
        {
            return self
                .settle_disconnected_archive(record, evidence_timeout)
                .await;
        }
        let status = match self.worker_status(record).await {
            Ok(status) => status,
            Err(error) if error.is_transport() => {
                return self
                    .settle_disconnected_archive(record, evidence_timeout)
                    .await;
            }
            Err(error) => return Err(error.into_error_body()),
        };
        if matches!(
            status.status,
            TerminalRuntimeStatus::Starting
                | TerminalRuntimeStatus::Running
                | TerminalRuntimeStatus::Finalizing
                | TerminalRuntimeStatus::OutputBlocked
        ) {
            return Err(ErrorBody::conflict(ARCHIVE_FINALIZATION_PENDING));
        }
        match self.retire_worker_classified(record).await {
            Ok(()) => {
                self.settle_disconnected_archive(record, evidence_timeout)
                    .await
            }
            Err(error) if error.may_settle_after_disconnect() => {
                self.settle_disconnected_archive(record, evidence_timeout)
                    .await
            }
            Err(error) => Err(error.into_error_body()),
        }
    }

    pub(crate) async fn settle_disconnected_archive(
        &self,
        record: &RuntimeRecord,
        timeout: Duration,
    ) -> std::result::Result<(), ErrorBody> {
        if self.runtime_is_settled(record, timeout).await {
            Ok(())
        } else {
            Err(worker_unreachable(ARCHIVE_RUNTIME_UNSETTLED))
        }
    }

    pub(crate) async fn settle_disconnected_termination(
        &self,
        record: &RuntimeRecord,
        force: bool,
        timeout: Duration,
    ) -> std::result::Result<bool, ErrorBody> {
        if !self.runtime_is_settled(record, timeout).await {
            return Ok(false);
        }
        self.persist_runtime_termination(record, force)?;
        Ok(true)
    }

    pub(crate) async fn runtime_is_settled(&self, record: &RuntimeRecord, timeout: Duration) -> bool {
        let identities_gone = wait_for_runtime_identities_exit(record, timeout).await;
        if !identities_gone || socket_may_be_serving(&record.socket_path).await {
            return false;
        }
        let current = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(record.terminal_id));
        let exit =
            self.read_namespace_json::<ExitRecord>(&self.paths.exit_record(record.terminal_id));
        let current_matches =
            matches!(current, Ok(current) if current.has_same_process_identity(record));
        let exit_matches = matches!(exit, Ok(exit) if exit.runtime_epoch == record.runtime_epoch);
        current_matches && (exit_matches || self.stable_termination_tombstone_matches(record))
    }
}

include!("lifecycle_archive_publication.rs");
