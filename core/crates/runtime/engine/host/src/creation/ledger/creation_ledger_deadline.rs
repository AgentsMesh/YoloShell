use super::*;

impl CreationLedger {
    pub(crate) fn reserve_before(
        &mut self,
        path: &Path,
        token: Uuid,
        spec: CreationSpec,
        receipts: &CreationReceiptStore,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<CreationBinding, ErrorBody> {
        self.ensure_healthy()?;
        let binding = self.new_binding(creation_token_digest(token), spec, receipts)?;
        let mut candidate = self.clone();
        candidate.bindings.push(binding.clone());
        self.install_before(path, candidate, deadline)?;
        Ok(binding)
    }

    pub(crate) fn recover_token_completion_before(
        &mut self,
        path: &Path,
        receipts: &CreationReceiptStore,
        token: Uuid,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(), ErrorBody> {
        self.ensure_healthy()?;
        let digest = creation_token_digest(token);
        let Some(index) = self
            .bindings
            .iter()
            .position(|binding| binding.creation_token_digest == digest)
        else {
            return Ok(());
        };
        let binding = &self.bindings[index];
        let Some(receipt) = receipts.lookup_binding(binding).map_err(internal)? else {
            return Ok(());
        };
        if matches!(
            receipt.completion,
            CreationCompletion::Finalized | CreationCompletion::Survivor
        ) && binding.phase != CreationPhase::Ready
        {
            return Err(ErrorBody::internal(CREATION_LIVE_RECEIPT_PHASE_CONFLICT));
        }
        receipts.publish_before(&receipt, deadline)?;
        let mut candidate = self.clone();
        candidate.bindings.remove(index);
        self.install_before(path, candidate, deadline)
    }

    pub(crate) fn prepare_launch_attempt_before(
        &mut self,
        path: &Path,
        result: CreateTerminalResult,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(CreationBinding, CreationLaunchIdentity), ErrorBody> {
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
        self.install_before(path, candidate, deadline)?;
        let attempt = self
            .bindings
            .iter()
            .find(|binding| binding.result == result)
            .cloned()
            .ok_or_else(creation_unauthorized)?;
        Ok((attempt, launch))
    }

    pub(crate) fn set_active_phase_before(
        &mut self,
        path: &Path,
        result: CreateTerminalResult,
        phase: CreationPhase,
        deadline: HostOperationDeadline,
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
        self.install_before(path, candidate, deadline)
    }
}
