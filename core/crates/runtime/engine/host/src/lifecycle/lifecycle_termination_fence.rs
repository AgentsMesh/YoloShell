impl LifecycleMutationFence {
    pub(crate) fn exact_terminate_request(
        &self,
        record: &RuntimeRecord,
        force: bool,
    ) -> std::result::Result<crate::worker::FencedTerminateRequest, ErrorBody> {
        self.runtime.validate_observed(Some(record))?;
        let LifecycleRuntimeFence::Selected(selected) = &self.runtime else {
            return Err(ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED));
        };
        Ok(crate::worker::FencedTerminateRequest {
            terminal_id: selected.terminal_id,
            runtime_epoch: selected.runtime_epoch,
            expected_status: self.selection.expected_status,
            force,
        })
    }
}

impl Host {
    pub(crate) async fn worker_termination_rpc(
        &self,
        record: &RuntimeRecord,
        force: bool,
        fence: Option<&LifecycleMutationFence>,
    ) -> std::result::Result<ResponseEnvelope, WorkerRpcError> {
        let request = match fence {
            Some(fence) => fence
                .exact_terminate_request(record, force)
                .map_err(WorkerRpcError::remote)?,
            None => crate::worker::FencedTerminateRequest {
                terminal_id: record.terminal_id,
                runtime_epoch: record.runtime_epoch,
                expected_status: None,
                force,
            },
        };
        self.custodied_worker_rpc(record, method::TERMINATE_RUNTIME_EXACT, &request)
            .await
    }

    pub(crate) async fn verify_completed_lifecycle_mutation(
        &self,
        terminal_id: Uuid,
        fence: &LifecycleMutationFence,
    ) -> std::result::Result<(), ErrorBody> {
        let lifecycle_lock = self.wait_for_resume_lifecycle_lock(terminal_id).await?;
        self.validate_lifecycle_catalog_fence(&fence.selection)
            .await?;
        self.require_current_lifecycle_runtime(terminal_id, &fence.runtime)?;
        self.require_no_pending_lifecycle_runtime(terminal_id, &fence.runtime)?;
        drop(lifecycle_lock);
        Ok(())
    }

    pub(crate) async fn acquire_retained_lifecycle_fence(
        &self,
        terminal_id: Uuid,
        fence: &LifecycleMutationFence,
    ) -> std::result::Result<crate::paths::private_file_lock::PrivateFileLock, ErrorBody> {
        let lifecycle_lock = self.wait_for_resume_lifecycle_lock(terminal_id).await?;
        self.validate_lifecycle_catalog_fence(&fence.selection)
            .await?;
        self.require_current_lifecycle_runtime(terminal_id, &fence.runtime)?;
        self.require_no_pending_lifecycle_runtime(terminal_id, &fence.runtime)?;
        Ok(lifecycle_lock)
    }
}
