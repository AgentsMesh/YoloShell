use super::*;

impl Host {
    pub(in crate::host) async fn create_session(
        &self,
        request: CreateTerminal,
    ) -> std::result::Result<CreateTerminalResult, ErrorBody> {
        let deadline = HostOperationDeadline::after(self.worker_ready_timeout);
        if request.creation_token.is_nil() {
            return Err(ErrorBody::invalid("creation token must not be nil"));
        }
        let request_fingerprint = CreationSpec::fingerprint(&request)?;
        let ledger_path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        require_creation_budget(deadline)?;
        let existing = self
            .existing_creation_before(
                &ledger_path,
                request.creation_token,
                &request_fingerprint,
                deadline,
            )
            .await?;
        let binding = match existing {
            Some(binding) => binding,
            None => {
                let spec = CreationSpec::resolve(&request, request_fingerprint)?;
                self.reserve_creation_before(&ledger_path, request.creation_token, spec, deadline)
                    .await?
            }
        };
        let _session_guard = deadline
            .timeout(self.session_locks.lock(binding.result.terminal_id))
            .await
            .ok_or_else(creation_launch_uncertain)?;
        require_creation_budget(deadline)?;
        let binding = self
            .existing_creation_before(
                &ledger_path,
                request.creation_token,
                &binding.spec.request_fingerprint,
                deadline,
            )
            .await?
            .ok_or_else(|| ErrorBody::internal("reserved creation disappeared"))?;
        require_creation_budget(deadline)?;
        let should_launch = self
            .creation_requires_launch_before(&ledger_path, &binding, deadline)
            .await?;
        if should_launch {
            let mut manifest = deadline
                .timeout(self.prepare_bound_session_before(&binding, deadline))
                .await
                .ok_or_else(creation_launch_uncertain)??;
            require_creation_budget(deadline)?;
            let (attempt, launch) = self
                .prepare_creation_launch_attempt_before(&ledger_path, binding.result, deadline)
                .await?;
            match self
                .launch_bound_session(&mut manifest, &attempt, &launch, deadline)
                .await
            {
                Ok(WorkerStart::Ready) => {
                    match self
                        .creation_evidence_before(&attempt, &launch, deadline)
                        .await?
                    {
                        CreationRuntimeEvidence::Ready => {}
                        CreationRuntimeEvidence::Mismatch => {
                            return Err(creation_runtime_mismatch());
                        }
                        _ => return Err(creation_launch_uncertain()),
                    }
                }
                Ok(WorkerStart::Uncertain(error)) => {
                    self.mark_creation_worker_start_uncertain_before(
                        &mut manifest,
                        &error,
                        deadline,
                    )
                    .await?;
                    require_creation_budget(deadline)?;
                    match self
                        .settle_creation_launch(&attempt, &launch, deadline)
                        .await?
                    {
                        CreationRuntimeEvidence::Ready => {}
                        CreationRuntimeEvidence::Mismatch => {
                            return Err(creation_runtime_mismatch());
                        }
                        CreationRuntimeEvidence::Failed(CreationFailureResolution::Resolved) => {
                            self.persist_creation_phase_before(
                                &ledger_path,
                                binding.result,
                                CreationPhase::SpawnFailed,
                                deadline,
                            )
                            .await?;
                            return Err(creation_launch_failed());
                        }
                        _ => return Err(creation_launch_uncertain()),
                    }
                }
                Err(error) => match self
                    .settle_creation_launch(&attempt, &launch, deadline)
                    .await?
                {
                    CreationRuntimeEvidence::Ready => {}
                    CreationRuntimeEvidence::Missing => {
                        if self
                            .creation_checkpoint_launch(&attempt, &launch)?
                            .is_none()
                        {
                            self.persist_creation_phase_before(
                                &ledger_path,
                                binding.result,
                                CreationPhase::SpawnFailed,
                                deadline,
                            )
                            .await?;
                        }
                        return Err(error);
                    }
                    CreationRuntimeEvidence::Failed(CreationFailureResolution::Resolved) => {
                        self.persist_creation_phase_before(
                            &ledger_path,
                            binding.result,
                            CreationPhase::SpawnFailed,
                            deadline,
                        )
                        .await?;
                        return Err(error);
                    }
                    _ => unreachable!("{}", CREATION_SETTLEMENT_INVARIANT),
                },
            }
            self.persist_creation_phase_before(
                &ledger_path,
                binding.result,
                CreationPhase::Ready,
                deadline,
            )
            .await?;
        }
        Ok(binding.result)
    }
}
