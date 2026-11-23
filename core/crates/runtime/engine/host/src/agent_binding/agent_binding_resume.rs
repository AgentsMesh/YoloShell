impl Host {
    pub(crate) async fn binding_for_resume(
        &self,
        manifest: &mut TerminalManifest,
        selection: ResumeSelection,
    ) -> std::result::Result<ResumeBinding, SessionResumeFailure> {
        let store = self.agent_binding_store(manifest.terminal_id);
        let ResumeSelection::Preserve(intent) = selection else {
            let path = store.path();
            let previous = self
                .capture_resume_binding_evidence(path)
                .map_err(internal)?;
            let codex_authority = self
                .capture_resume_codex_authority_evidence(manifest.terminal_id, &store)
                .map_err(internal)?;
            return Ok(ResumeBinding::Shell(ResumeBindingDisposition::Clear {
                operation_id: Uuid::new_v4(),
                previous,
                codex_authority,
            }));
        };
        let binding = match self
            .repair_native_binding_for_resume(manifest)
            .and_then(|()| self.load_agent_binding(manifest))
        {
            Ok(binding) => binding,
            Err(error) => {
                let failure = SessionResumeFailure::Protocol(ErrorBody::new(
                    crate::agent::constants::error::BINDING_INVALID,
                    format!("{AGENT_BINDING_LOAD_FAILED}: {error}"),
                    false,
                ));
                return Err(self
                    .persist_classified_resume_failure(manifest, RecoveryState::Uncertain, failure)
                    .await);
            }
        };
        let Some(binding) = binding else {
            let failure = SessionResumeFailure::TerminalRecreate(ErrorBody::new(
                crate::protocol::error_code::RECREATE_REQUIRED,
                RECREATE_REQUIRED_DETAIL,
                false,
            ));
            return Err(self
                .persist_classified_resume_failure(manifest, RecoveryState::NeedsRecreate, failure)
                .await);
        };
        match intent.policy(&binding) {
            ResumePolicy::Provider => {
                self.authorize_native_resume(manifest, &binding)
                    .map_err(internal)?;
                Ok(ResumeBinding::Provider(binding))
            }
            ResumePolicy::ExplicitOnly => {
                let failure = SessionResumeFailure::TerminalRecreate(ErrorBody::new(
                    crate::protocol::error_code::RECREATE_REQUIRED,
                    EXPLICIT_PROVIDER_RESUME_REQUIRED_DETAIL,
                    false,
                ));
                Err(self
                    .persist_classified_resume_failure(
                        manifest,
                        RecoveryState::NeedsRecreate,
                        failure,
                    )
                    .await)
            }
            ResumePolicy::FailClosed => {
                let failure = SessionResumeFailure::TerminalRecreate(ErrorBody::new(
                    crate::protocol::error_code::AGENT_RECOVERY_UNAVAILABLE,
                    AGENT_RECOVERY_UNAVAILABLE_DETAIL,
                    false,
                ));
                Err(self
                    .persist_classified_resume_failure(
                        manifest,
                        RecoveryState::NeedsRecreate,
                        failure,
                    )
                    .await)
            }
        }
    }
}
