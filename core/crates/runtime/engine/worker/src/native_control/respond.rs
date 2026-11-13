//! Delivers one typed response to a pending provider interaction. The pending
//! request is owned by the hook connection; this path resolves it under the
//! same controller, lease, witness and durable intent checks as other controls.

use super::{
    NativeControlOutcome, NativeControlResult, NativeRespondRequest, constants::*,
    observation::journal_error, outcome_for_receipt, source,
};
use crate::protocol::ErrorBody;
use crate::worker::{
    AuthenticatedController, WorkerState, agent_monitor_publication_fence, lock_recover,
};
use std::sync::Arc;
use yolo_agent_control_contract::NativeControlWitness;
use yolo_agent_control_journal::{Completion, ReservationResult};

impl WorkerState {
    pub(in crate::worker) async fn respond_native_agent(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        request: &NativeRespondRequest,
    ) -> Result<NativeControlResult, ErrorBody> {
        self.validate_runtime(request.terminal_id, request.runtime_epoch)?;
        let witness = request.witness.fields();
        if request.request_id.is_nil()
            || request.interaction.is_nil()
            || witness.terminal_id != request.terminal_id
            || witness.runtime_epoch != request.runtime_epoch
            || !matches!(request.choice.as_str(), "allow" | "deny")
        {
            return Err(ErrorBody::invalid(INVALID_REQUEST));
        }
        let allow = request.choice == "allow";
        let _agent = self.agent_generation_gate.lock().await;
        let _permit = self
            .admit_writer_mutation(controller, request.generation, &request.writer_capability)
            .await?;
        let record = lock_recover(&self.record).clone();
        if witness.worker_id != record.worker_id || witness.boot_id != record.boot_id {
            return Err(ErrorBody::conflict(RUNTIME_CHANGED));
        }
        {
            let control = lock_recover(&self.native_control);
            let control = control
                .as_ref()
                .ok_or_else(|| ErrorBody::conflict(UNAVAILABLE))?;
            if let Some(receipt) = control
                .lookup(&record, request.request_id, request.command_digest)
                .map_err(journal_error)?
            {
                return Ok(result(request, outcome_for_receipt(receipt)));
            }
        }
        let fence = agent_monitor_publication_fence::try_acquire(self, &record)
            .map_err(|_| ErrorBody::conflict(SOURCE_CHANGED))?;
        let outcome = {
            let mut control_guard = lock_recover(&self.native_control);
            let control = control_guard
                .as_mut()
                .ok_or_else(|| ErrorBody::conflict(UNAVAILABLE))?;
            source::with_source(
                self,
                fence.durable(),
                Some(&request.witness),
                |_binding, witness, verify_source| {
                    fence.verify()?;
                    verify_source()?;
                    self.authorize_control(
                        controller,
                        request.generation,
                        &request.writer_capability,
                    )?;
                    let reserved =
                        control.reserve(&record, request.request_id, request.command_digest)?;
                    let ReservationResult::New(reservation) = reserved else {
                        let ReservationResult::Existing(receipt) = reserved else {
                            unreachable!()
                        };
                        return Ok(outcome_for_receipt(receipt));
                    };
                    #[cfg(test)]
                    self.native_control_test_hooks.run_after_reserve();
                    if control.verify(&record).is_err()
                        || fence.verify().is_err()
                        || verify_source().is_err()
                    {
                        return Ok(NativeControlOutcome::Unknown);
                    }
                    let admitted = self
                        .authorize_control(
                            controller,
                            request.generation,
                            &request.writer_capability,
                        )
                        .is_ok();
                    let delivered =
                        admitted && self.resolve_interaction(request.interaction, witness, allow);
                    let completion = if delivered {
                        Completion::Delivered
                    } else {
                        Completion::NotDelivered
                    };
                    Ok(control
                        .complete(&record, reservation, completion)
                        .map(outcome_for_receipt)
                        .unwrap_or(NativeControlOutcome::Unknown))
                },
            )
            .map_err(|_| ErrorBody::conflict(SOURCE_CHANGED))?
        };
        Ok(result(request, outcome))
    }
}

fn result(request: &NativeRespondRequest, outcome: NativeControlOutcome) -> NativeControlResult {
    NativeControlResult {
        terminal_id: request.terminal_id,
        runtime_epoch: request.runtime_epoch,
        request_id: request.request_id,
        command_digest: request.command_digest,
        outcome,
    }
}
