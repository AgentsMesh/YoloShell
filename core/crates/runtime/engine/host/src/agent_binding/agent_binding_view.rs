impl Host {
    pub(crate) async fn binding_for_view_with_observation(
        &self,
        manifest: &mut TerminalManifest,
        observation: Option<&crate::runtime_observation::RuntimeObservation>,
    ) -> (Option<AgentBinding>, Option<Uuid>) {
        let observed_runtime = observation.and_then(|value| value.observed_record.as_ref());
        let loaded = match self.load_agent_binding_with_observed_runtime(manifest, observed_runtime) {
            Ok(binding) => binding,
            Err(error) => {
                tracing::error!(
                    terminal_id = %manifest.terminal_id,
                    %error,
                    "invalid agent binding is isolated from the session view"
                );
                if !manifest.archived {
                    manifest.recovery_state = RecoveryState::Uncertain;
                }
                return (None, None);
            }
        };
        loaded
    }
}
