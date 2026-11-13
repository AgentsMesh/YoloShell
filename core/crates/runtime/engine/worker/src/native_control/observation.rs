use super::{
    NativeControlLookupRequest, NativeControlResult, NativeObservationRequest, constants::*,
    outcome_for_receipt, source,
};
use crate::agent::AgentState;
use crate::protocol::ErrorBody;
use crate::worker::{
    AuthenticatedController, WorkerState, agent_monitor_publication_fence, lock_recover,
};
use std::sync::Arc;
use yolo_agent_control_contract::{NativeControlObservation, NativeTurnEnd, NativeTurnEndKind};
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
        let observed =
            source::with_source(self, fence.durable(), None, |binding, witness, verify| {
                anyhow::ensure!(
                    witness.fields().binding_digest == request.binding_digest,
                    SOURCE_CHANGED
                );
                fence.verify()?;
                verify()?;
                // Readiness is a provider fact; terminal mode is a submit-time
                // transport precondition, never a capability. Only a stop
                // candidate needs the structured turn-ending record.
                let (ready, turn_end) = match binding.state {
                    AgentState::WaitingForInput => (true, None),
                    AgentState::Active => {
                        let end = turn_end(self, binding);
                        (end.is_some(), end)
                    }
                    _ => (false, None),
                };
                let interaction = self.observed_interaction(witness);
                Ok(NativeControlObservation::new_with_interaction(
                    binding.clone(),
                    witness.clone(),
                    ready,
                    turn_end,
                    interaction,
                )?)
            });
        Ok(observed.ok())
    }
}

fn turn_end(state: &WorkerState, binding: &crate::agent::AgentBinding) -> Option<NativeTurnEnd> {
    let probe = state
        .provider_runtime
        .get(&binding.kind)?
        .turn_end_probe()?;
    let end = probe.read_turn_end(binding).ok()??;
    Some(NativeTurnEnd {
        turn_id: end.turn_id,
        kind: match end.kind {
            yolo_agent_providers::TurnEndKind::Completed => NativeTurnEndKind::Completed,
            yolo_agent_providers::TurnEndKind::Aborted => NativeTurnEndKind::Aborted,
        },
    })
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
