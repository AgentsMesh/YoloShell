use super::{
    NativeControlLookupRequest, NativeControlResult, NativeObservationRequest, constants::*,
    outcome_for_receipt, source,
};
use crate::protocol::ErrorBody;
use crate::worker::{
    AuthenticatedController, WorkerState, agent_monitor_publication_fence, lock_recover,
};
use std::sync::Arc;
use yolo_agent_control_contract::NativeControlObservation;
use yolo_agent_control_journal::JournalError;

impl WorkerState {
    pub(in crate::worker) async fn observe_native_agent(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        request: &NativeObservationRequest,
    ) -> Result<Option<NativeControlObservation>, ErrorBody> {
        self.validate_runtime(request.terminal_id, request.runtime_epoch)?;
        let _agent = self.agent_generation_gate.lock().await;
        let _permit = self.admit_controller_mutation(controller).await?;
        self.ensure_ready()?;
        self.ensure_process_running()?;
        let record = lock_recover(&self.record).clone();
        let control = lock_recover(&self.native_control);
        if !control
            .as_ref()
            .is_some_and(|control| control.verify(&record).is_ok())
        {
            return Ok(None);
        }
        let Ok(fence) = agent_monitor_publication_fence::try_acquire(self, &record) else {
            return Ok(None);
        };
        let observed = source::with_source(
            self,
            fence.durable(),
            None,
            |binding, witness, stop_turn, verify| {
                anyhow::ensure!(
                    witness.fields().binding_digest == request.binding_digest,
                    SOURCE_CHANGED
                );
                fence.verify()?;
                verify()?;
                let (ready, turn_end) = super::prompt_readiness::observe(self, binding, stop_turn);
                let interaction = self.observed_interaction(witness);
                Ok(NativeControlObservation::new_with_interaction(
                    binding.clone(),
                    witness.clone(),
                    ready,
                    turn_end,
                    interaction,
                )?)
            },
        );
        Ok(observed.ok())
    }
}

impl WorkerState {
    pub(in crate::worker) async fn native_control_receipt(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        request: &NativeControlLookupRequest,
    ) -> Result<Option<NativeControlResult>, ErrorBody> {
        self.validate_runtime(request.terminal_id, request.runtime_epoch)?;
        // Receipt absence must be observed after previously queued effects. A
        // failed source refresh cannot substitute for crossing this barrier.
        let _agent = self.agent_generation_gate.lock().await;
        let _permit = self.admit_controller_mutation(controller).await?;
        let record = lock_recover(&self.record).clone();
        let control = lock_recover(&self.native_control);
        let control = control
            .as_ref()
            .ok_or_else(|| ErrorBody::conflict(UNAVAILABLE))?;
        let receipt = control
            .lookup(&record, request.request_id, request.command_digest)
            .map_err(journal_error)?;
        Ok(receipt.map(|receipt| NativeControlResult {
            terminal_id: request.terminal_id,
            runtime_epoch: request.runtime_epoch,
            request_id: request.request_id,
            command_digest: request.command_digest,
            outcome: outcome_for_receipt(receipt),
        }))
    }
}

pub(super) fn journal_error(error: JournalError) -> ErrorBody {
    if matches!(error, JournalError::Conflict) {
        ErrorBody::conflict(REQUEST_CONFLICT)
    } else {
        ErrorBody::internal(JOURNAL_FAILED)
    }
}
