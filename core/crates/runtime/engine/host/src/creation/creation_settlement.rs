use super::*;

impl Host {
    #[cfg(test)]
    pub(crate) async fn creation_requires_launch(
        &self,
        ledger_path: &Path,
        binding: &CreationBinding,
    ) -> std::result::Result<bool, ErrorBody> {
        let deadline = HostOperationDeadline::after(self.worker_ready_timeout);
        self.creation_requires_launch_before(ledger_path, binding, deadline)
            .await
    }

    pub(crate) async fn creation_requires_launch_before(
        &self,
        ledger_path: &Path,
        binding: &CreationBinding,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<bool, ErrorBody> {
        require_creation_budget(deadline)?;
        match binding.phase {
            CreationPhase::Ready => {
                deadline
                    .timeout(self.ensure_created_session_exists_before(binding, deadline))
                    .await
                    .ok_or_else(creation_launch_uncertain)??;
                require_creation_budget(deadline)?;
                Ok(false)
            }
            CreationPhase::Reserved => Ok(true),
            CreationPhase::Launching | CreationPhase::SpawnFailed => {
                deadline
                    .timeout(self.existing_bound_manifest_before(binding, deadline))
                    .await
                    .ok_or_else(creation_launch_uncertain)??
                    .ok_or_else(|| ErrorBody::internal(CREATION_SESSION_MISSING))?;
                require_creation_budget(deadline)?;
                let planned = binding
                    .launch
                    .as_ref()
                    .ok_or_else(|| ErrorBody::internal(CREATION_PLAN_MISSING))?;
                match self
                    .settle_creation_launch(binding, planned, deadline)
                    .await?
                {
                    CreationRuntimeEvidence::Ready => {
                        self.persist_creation_phase_before(
                            ledger_path,
                            binding.result,
                            CreationPhase::Ready,
                            deadline,
                        )
                        .await?;
                        Ok(false)
                    }
                    CreationRuntimeEvidence::Missing => Ok(true),
                    CreationRuntimeEvidence::Failed(CreationFailureResolution::Resolved)
                        if binding.phase == CreationPhase::SpawnFailed =>
                    {
                        Ok(true)
                    }
                    CreationRuntimeEvidence::Failed(CreationFailureResolution::Resolved) => {
                        self.persist_creation_phase_before(
                            ledger_path,
                            binding.result,
                            CreationPhase::SpawnFailed,
                            deadline,
                        )
                        .await?;
                        Err(creation_launch_failed())
                    }
                    _ => unreachable!("{}", CREATION_SETTLEMENT_INVARIANT),
                }
            }
            CreationPhase::Terminated => unreachable!("confirm rejects terminated bindings"),
        }
    }

    pub(in crate::host) async fn ensure_created_session_exists(
        &self,
        binding: &CreationBinding,
    ) -> std::result::Result<(), ErrorBody> {
        self.ensure_created_session_exists_with_deadline(binding, None)
            .await
    }

    pub(crate) async fn ensure_created_session_exists_before(
        &self,
        binding: &CreationBinding,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(), ErrorBody> {
        self.ensure_created_session_exists_with_deadline(binding, Some(deadline))
            .await
    }

    pub(crate) async fn ensure_created_session_exists_with_deadline(
        &self,
        binding: &CreationBinding,
        deadline: Option<HostOperationDeadline>,
    ) -> std::result::Result<(), ErrorBody> {
        let manifest = match deadline {
            Some(deadline) => {
                self.existing_bound_manifest_before(binding, deadline)
                    .await?
            }
            None => self.existing_bound_manifest(binding).await?,
        };
        manifest
            .ok_or_else(|| ErrorBody::internal("ready creation is missing its durable session"))?;
        let launch = binding
            .launch
            .as_ref()
            .ok_or_else(|| ErrorBody::internal("ready creation is missing its launch identity"))?;
        match self.creation_runtime_evidence(binding, launch) {
            CreationRuntimeEvidence::Ready => Ok(()),
            CreationRuntimeEvidence::Mismatch => Err(creation_runtime_mismatch()),
            _ => Err(creation_launch_uncertain()),
        }
    }

    pub(crate) async fn settle_creation_launch(
        &self,
        binding: &CreationBinding,
        launch: &CreationLaunchIdentity,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<CreationRuntimeEvidence, ErrorBody> {
        loop {
            require_creation_budget(deadline)?;
            if self.reconcile_pre_runtime_checkpoint(binding, launch)? {
                return Ok(CreationRuntimeEvidence::Missing);
            }
            let evidence = self
                .creation_evidence_before(binding, launch, deadline)
                .await?;
            match evidence {
                CreationRuntimeEvidence::Missing
                | CreationRuntimeEvidence::Ready
                | CreationRuntimeEvidence::Failed(CreationFailureResolution::Resolved) => {
                    return Ok(evidence);
                }
                CreationRuntimeEvidence::Mismatch => return Err(creation_runtime_mismatch()),
                CreationRuntimeEvidence::MatchingPending
                | CreationRuntimeEvidence::Failed(CreationFailureResolution::Active)
                | CreationRuntimeEvidence::Unknown => {}
            }
            deadline
                .wait_for_poll(self.worker_ready_poll_interval)
                .await;
        }
    }

    pub(crate) async fn creation_evidence_before(
        &self,
        binding: &CreationBinding,
        launch: &CreationLaunchIdentity,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<CreationRuntimeEvidence, ErrorBody> {
        require_creation_budget(deadline)?;
        deadline
            .timeout(self.authenticated_creation_runtime_evidence(binding, launch))
            .await
            .ok_or_else(creation_launch_uncertain)
    }
}

pub(in crate::host) fn require_creation_budget(
    deadline: HostOperationDeadline,
) -> std::result::Result<(), ErrorBody> {
    if deadline.is_elapsed() {
        Err(creation_launch_uncertain())
    } else {
        Ok(())
    }
}

include!("creation_checkpoint_settlement.rs");
