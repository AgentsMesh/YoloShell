use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::host) enum CleanupCreationState {
    Ready,
    Terminated,
    Archived,
    Finalized,
    Survivor,
}

impl CreationPhase {
    pub(crate) fn permits_launch(self) -> bool {
        matches!(self, Self::Reserved | Self::Launching | Self::SpawnFailed)
    }

    pub(crate) fn permits_transition(self, next: Self) -> bool {
        self == next
            || matches!(
                (self, next),
                (
                    Self::Launching,
                    Self::SpawnFailed | Self::Ready | Self::Terminated
                ) | (Self::SpawnFailed, Self::Ready | Self::Terminated)
                    | (Self::Ready, Self::Terminated)
            )
    }
}

impl CreationLedger {
    pub(crate) fn ensure_healthy(&self) -> std::result::Result<(), ErrorBody> {
        if self.poisoned {
            Err(ErrorBody::internal(
                "creation ledger persistence is uncertain; restart the host before retrying",
            ))
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    pub(crate) fn prepare_launch(
        &mut self,
        path: &Path,
        result: CreateTerminalResult,
    ) -> std::result::Result<CreationLaunchIdentity, ErrorBody> {
        self.ensure_healthy()?;
        let mut candidate = self.clone();
        let binding = candidate
            .bindings
            .iter_mut()
            .find(|entry| entry.result == result)
            .ok_or_else(creation_unauthorized)?;
        if !binding.phase.permits_launch() {
            return Err(ErrorBody::conflict(CREATION_LAUNCH_PHASE_CONFLICT));
        }
        let launch = if binding.phase == CreationPhase::SpawnFailed {
            binding.launch_generation = binding
                .launch_generation
                .checked_add(INITIAL_CREATION_LAUNCH_GENERATION)
                .ok_or_else(|| ErrorBody::internal(CREATION_LAUNCH_GENERATION_EXHAUSTED))?;
            let launch = CreationLaunchIdentity::new();
            binding.launch = Some(launch.clone());
            launch
        } else {
            if binding.phase == CreationPhase::Reserved {
                binding.launch_generation = INITIAL_CREATION_LAUNCH_GENERATION;
            }
            binding
                .launch
                .get_or_insert_with(CreationLaunchIdentity::new)
                .clone()
        };
        binding.phase = CreationPhase::Launching;
        self.install(path, candidate)?;
        Ok(launch)
    }

    #[cfg(test)]
    pub(crate) fn prepare_launch_attempt(
        &mut self,
        path: &Path,
        result: CreateTerminalResult,
    ) -> std::result::Result<(CreationBinding, CreationLaunchIdentity), ErrorBody> {
        let launch = self.prepare_launch(path, result)?;
        let attempt = self
            .bindings
            .iter()
            .find(|binding| binding.result == result)
            .cloned()
            .ok_or_else(creation_unauthorized)?;
        Ok((attempt, launch))
    }

    pub(in crate::host) fn set_active_phase(
        &mut self,
        path: &Path,
        result: CreateTerminalResult,
        phase: CreationPhase,
    ) -> std::result::Result<(), ErrorBody> {
        self.ensure_healthy()?;
        let mut candidate = self.clone();
        let binding = candidate
            .bindings
            .iter_mut()
            .find(|entry| entry.result == result)
            .ok_or_else(creation_unauthorized)?;
        if !binding.phase.permits_transition(phase) {
            return Err(ErrorBody::conflict(CREATION_PHASE_TRANSITION_CONFLICT));
        }
        binding.phase = phase;
        self.install(path, candidate)
    }

    pub(in crate::host) fn cleanup_state(
        &mut self,
        path: &Path,
        terminal_id: Uuid,
        capability: Uuid,
        receipts: &CreationReceiptStore,
    ) -> std::result::Result<CleanupCreationState, ErrorBody> {
        self.ensure_healthy()?;
        if let Some(index) = self
            .bindings
            .iter()
            .position(|entry| entry.result.terminal_id == terminal_id)
        {
            let binding = &self.bindings[index];
            if binding.result.creation_capability != capability {
                return Err(creation_unauthorized());
            }
            let phase = binding.phase;
            if let Some(completion) =
                self.recover_binding_completion(path, receipts, Some(index))?
            {
                return Ok(match completion {
                    CreationCompletion::Archived => CleanupCreationState::Archived,
                    CreationCompletion::Finalized => CleanupCreationState::Finalized,
                    CreationCompletion::Survivor => CleanupCreationState::Survivor,
                });
            }
            return match phase {
                CreationPhase::Ready => Ok(CleanupCreationState::Ready),
                CreationPhase::Terminated => Ok(CleanupCreationState::Terminated),
                _ => Err(ErrorBody::conflict("creation is not ready for cleanup")),
            };
        }
        let receipt = receipts
            .lookup_session(terminal_id)
            .map_err(internal)?
            .ok_or_else(creation_unauthorized)?;
        if receipt.result.creation_capability != capability {
            return Err(creation_unauthorized());
        }
        Ok(match receipt.completion {
            CreationCompletion::Archived => CleanupCreationState::Archived,
            CreationCompletion::Finalized => CleanupCreationState::Finalized,
            CreationCompletion::Survivor => CleanupCreationState::Survivor,
        })
    }

    pub(in crate::host) fn complete_archive(
        &mut self,
        path: &Path,
        result: CreateTerminalResult,
        receipts: &CreationReceiptStore,
    ) -> std::result::Result<(), ErrorBody> {
        self.ensure_healthy()?;
        let mut candidate = self.clone();
        let index = candidate
            .bindings
            .iter()
            .position(|entry| entry.result == result)
            .ok_or_else(creation_unauthorized)?;
        if candidate.bindings[index].phase != CreationPhase::Terminated {
            return Err(ErrorBody::conflict(
                "creation must be terminated before it can be archived",
            ));
        }
        let receipt =
            CreationReceipt::from_binding(&candidate.bindings[index], CreationCompletion::Archived);
        receipts.publish(&receipt).map_err(internal)?;
        candidate.bindings.remove(index);
        self.install(path, candidate)
    }
}

pub(crate) fn creation_unauthorized() -> ErrorBody {
    ErrorBody::new(
        crate::protocol::error_code::UNAUTHORIZED,
        "creation capability does not authorize this session",
        false,
    )
}
