impl Host {
    pub(crate) async fn terminate_bound_session(
        &self,
        terminal_id: Uuid,
        force: bool,
    ) -> std::result::Result<(), ErrorBody> {
        self.terminate_bound_session_with_fence(terminal_id, force, None)
            .await
    }

    pub(crate) async fn terminate_bound_session_with_fence(
        &self,
        terminal_id: Uuid,
        force: bool,
        runtime_fence: Option<&LifecycleMutationFence>,
    ) -> std::result::Result<(), ErrorBody> {
        let manifest = self
            .catalog_get(terminal_id)
            .await
            .map_err(internal)?
            .ok_or_else(|| ErrorBody::not_found(SESSION_NOT_FOUND))?;
        if manifest.archived {
            return Ok(());
        }
        let record = match self.live_runtime_record(terminal_id).await {
            Ok(record) => record,
            Err(error) => {
                if self
                    .settle_unreachable_runtime_termination(terminal_id, force, runtime_fence)
                    .await?
                {
                    return Ok(());
                }
                return Err(error);
            }
        };
        if let Some(fence) = runtime_fence {
            fence.runtime.validate_observed(Some(&record))?;
        }
        if self.current_child_exit_is_durable(&record) {
            self.persist_runtime_termination(&record, force)?;
            return Ok(());
        }
        let response = match self
            .worker_termination_rpc(&record, force, runtime_fence)
            .await
        {
            Ok(response) => response,
            Err(error) => {
                let error = error.into_error_body();
                if self
                    .settle_unreachable_runtime_termination(terminal_id, force, runtime_fence)
                    .await?
                {
                    return Ok(());
                }
                return Err(error);
            }
        };
        if response.ok {
            Ok(())
        } else {
            let error = response
                .error
                .unwrap_or_else(|| ErrorBody::internal(WORKER_TERMINATION_FAILED));
            self.settle_deferred_termination(&record, error).await
        }
    }

    pub(crate) async fn settle_inactive_bound_session(
        &self,
        terminal_id: Uuid,
        force: bool,
        runtime_fence: Option<&LifecycleMutationFence>,
    ) -> std::result::Result<(), ErrorBody> {
        let manifest = self
            .catalog_get(terminal_id)
            .await
            .map_err(internal)?
            .ok_or_else(|| ErrorBody::not_found(SESSION_NOT_FOUND))?;
        if manifest.archived {
            return Ok(());
        }
        let record = match self.live_runtime_record(terminal_id).await {
            Ok(record) => record,
            Err(error) => {
                if let Some(fence) = runtime_fence
                    && matches!(&fence.runtime, LifecycleRuntimeFence::Absent)
                {
                    self.require_current_lifecycle_runtime(terminal_id, &fence.runtime)?;
                    return Ok(());
                }
                if self
                    .settle_unreachable_runtime_termination(terminal_id, force, runtime_fence)
                    .await?
                {
                    return Ok(());
                }
                return Err(error);
            }
        };
        if let Some(fence) = runtime_fence {
            fence.runtime.validate_observed(Some(&record))?;
        }
        if self.current_child_exit_is_durable(&record) {
            return Ok(());
        }
        match self.worker_status(&record).await {
            Ok(status) if status.status == TerminalRuntimeStatus::Exited => Ok(()),
            Ok(_) => Err(ErrorBody::conflict(LIFECYCLE_ACTIVE_SESSION)),
            Err(error) => {
                let error = error.into_error_body();
                if self
                    .settle_unreachable_runtime_termination(terminal_id, force, runtime_fence)
                    .await?
                {
                    Ok(())
                } else {
                    Err(error)
                }
            }
        }
    }

    pub(crate) async fn wait_for_session_exit(&self, terminal_id: Uuid) -> std::result::Result<(), ErrorBody> {
        let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
        loop {
            match self.live_runtime_record(terminal_id).await {
                Ok(record) => {
                    if self.current_child_exit_is_durable(&record) {
                        return Ok(());
                    }
                    match self.worker_status(&record).await {
                        Ok(status) if status.status == TerminalRuntimeStatus::Exited => return Ok(()),
                        Ok(_) => {}
                        Err(error) => {
                            let remaining =
                                deadline.saturating_duration_since(tokio::time::Instant::now());
                            if wait_for_runtime_identities_exit(&record, remaining).await {
                                return Ok(());
                            }
                            return Err(error.into_error_body());
                        }
                    }
                }
                Err(error) if self.runtime_record_may_be_live(terminal_id) => return Err(error),
                Err(_) => return Ok(()),
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(ErrorBody::new(
                    crate::protocol::error_code::WORKER_UNREACHABLE,
                    SESSION_EXIT_DEADLINE_EXCEEDED,
                    true,
                ));
            }
            tokio::time::sleep(PROCESS_EXIT_POLL_INTERVAL).await;
        }
    }

    pub(crate) fn runtime_record_may_be_live(&self, terminal_id: Uuid) -> bool {
        let path = self.paths.runtime_record(terminal_id);
        let Ok(record) = self.read_namespace_json::<RuntimeRecord>(&path) else {
            return path.try_exists().unwrap_or(true);
        };
        if !runtime_loss::runtime_record_identity_is_verifiable(&record, terminal_id) {
            return true;
        }
        !process_identity_is_gone(record.worker_pid, record.worker_start_time)
            || !process_identity_is_gone(record.child_pid, record.child_start_time)
    }
}

include!("lifecycle_archive.rs");
include!("lifecycle_deferred_termination.rs");
include!("lifecycle_resume_lock_wait.rs");
include!("lifecycle_selection.rs");
include!("lifecycle_selection_projection.rs");
include!("lifecycle_runtime_fence.rs");
include!("lifecycle_termination_fence.rs");
