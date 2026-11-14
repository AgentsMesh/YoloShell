impl Host {
    pub(crate) async fn binding_for_view(
        &self,
        manifest: &mut TerminalManifest,
    ) -> (Option<AgentBinding>, Option<Uuid>) {
        match self.load_agent_binding_with_authority(manifest) {
            Ok(binding) => binding,
            Err(error) => {
                tracing::error!(
                    terminal_id = %manifest.terminal_id,
                    %error,
                    "invalid agent binding is isolated from the session view"
                );
                // A read path must never publish lifecycle state: GET/LIST run
                // without the per-terminal session lock, and a durable repair
                // here can invalidate a concurrent resume or archive digest
                // fence after it has retired the previous Worker. Reflect the
                // repair in this view only; reconcile owns durable repair
                // under the session lock.
                if !manifest.archived {
                    manifest.recovery_state = RecoveryState::Uncertain;
                }
                (None, None)
            }
        }
    }
}
