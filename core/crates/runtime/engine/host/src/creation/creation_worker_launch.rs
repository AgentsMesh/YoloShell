use super::*;

impl Host {
    pub(crate) async fn launch_bound_session(
        &self,
        manifest: &mut TerminalManifest,
        binding: &CreationBinding,
        launch: &CreationLaunchIdentity,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<WorkerStart, ErrorBody> {
        let readiness_deadline = deadline.reserving(
            crate::session_timing::SESSION_DURABLE_STARTUP_SETTLEMENT_MARGIN,
        );
        self.spawn_creation_worker(manifest, binding, launch, readiness_deadline)
            .await
            .map_err(|error| {
                if error.is::<CreationPublicationDeadlineExpired>() {
                    creation_launch_uncertain()
                } else {
                    worker_start_failure(manifest.terminal_id, &error)
                }
            })
    }
}
