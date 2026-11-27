use super::*;

impl Host {
    pub(super) fn project_runtime_recovery_state(
        &self,
        manifest: &mut TerminalManifest,
        observation: Option<&RuntimeObservation>,
    ) {
        let Some(observation) = observation else {
            return;
        };
        let runtime = &observation.runtime;
        if manifest.archived
            || manifest.recovery_state != RecoveryState::Reattached
            || observation.is_authenticated()
            || runtime.status != TerminalRuntimeStatus::Unreachable
            || !process_identity_is_gone(runtime.worker_pid, runtime.worker_start_time)
            || self.stable_termination_view(manifest, runtime, observation.observed_record.as_ref())
        {
            return;
        }
        manifest.recovery_state = RecoveryState::Uncertain;
    }

    pub(super) fn stable_termination_view(
        &self,
        manifest: &TerminalManifest,
        runtime: &RuntimeView,
        observed_record: Option<&RuntimeRecord>,
    ) -> bool {
        observed_record
            .cloned()
            .or_else(|| {
                self.read_namespace_json::<RuntimeRecord>(
                    &self.paths.runtime_record(manifest.terminal_id),
                )
                .ok()
            })
            .is_some_and(|record| {
                runtime_view_matches(runtime, &record)
                    && self.stable_user_termination_matches(manifest, &record)
            })
    }
}
