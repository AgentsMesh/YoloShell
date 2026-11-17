pub(crate) struct TerminationIntent {
    pub(crate) terminal_id: Uuid,
    pub(crate) force: bool,
}

impl Host {
    pub(crate) async fn terminate_product_runtime(
        &self,
        request: &TerminationIntent,
        inactive_only: bool,
        runtime_fence: Option<&LifecycleMutationFence>,
    ) -> std::result::Result<(), ErrorBody> {
        if inactive_only {
            self.settle_inactive_bound_session(request.terminal_id, request.force, runtime_fence)
                .await?;
        } else {
            self.terminate_bound_session_with_fence(
                request.terminal_id,
                request.force,
                runtime_fence,
            )
            .await?;
        }
        if let Some(fence) = runtime_fence {
            self.verify_completed_lifecycle_mutation(request.terminal_id, fence)
                .await?;
        }
        Ok(())
    }
}
