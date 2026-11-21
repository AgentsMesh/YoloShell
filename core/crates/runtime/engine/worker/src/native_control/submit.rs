use super::{
    NativeControlOutcome, NativeControlResult, NativeSubmitRequest, constants::*,
    outcome_for_receipt, source,
};
use crate::protocol::ErrorBody;
use crate::worker::{
    AuthenticatedController, WorkerState, agent_monitor_publication_fence, lock_recover,
};
use std::sync::Arc;
use yolo_agent_control_journal::{Completion, ReservationResult};

const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";
const SUBMIT: &[u8] = b"\r";
const PASTE_END_AND_SUBMIT: &[u8] = b"\x1b[201~\r";

impl WorkerState {
    pub(in crate::worker) async fn submit_native_agent(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        request: &NativeSubmitRequest,
    ) -> Result<NativeControlResult, ErrorBody> {
        self.validate_runtime(request.terminal_id, request.runtime_epoch)?;
        let witness = request.witness.fields();
        if request.request_id.is_nil()
            || witness.terminal_id != request.terminal_id
            || witness.runtime_epoch != request.runtime_epoch
        {
            return Err(ErrorBody::invalid(INVALID_REQUEST));
        }
        let since_ms = crate::model::now_ms();
        let mut effects = self.events.subscribe();
        let mut expect_turn = false;
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
                .map_err(super::observation::journal_error)?
            {
                return Ok(submit_result(request, outcome_for_receipt(receipt)));
            }
        }
        let input = self
            .observe_prompt_input()
            .await
            .ok_or_else(|| ErrorBody::conflict(SOURCE_CHANGED))?;
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
            |binding, witness, verify| {
                let ready = || -> anyhow::Result<()> {
                    fence.verify()?;
                    verify()?;
                    self.authorize_control(
                        controller,
                        request.generation,
                        &request.writer_capability,
                    )?;
                    let _ = input.verify(self, binding, witness)?;
                    Ok(())
                };
                ready()?;
                let gap = self
                    .provider_runtime
                    .get(&binding.kind)
                    .map_or(std::time::Duration::ZERO, |provider| {
                        provider.prompt_submit().enter_gap()
                    });
                let pasted = [PASTE_START, request.prompt.text().as_bytes()].concat();
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
                // Durable intent precedes the writes. Partial writes stay unknown;
                // neither a reconnect nor a repeated request is permission to submit twice.
                if control.verify(&record).is_err() || ready().is_err() {
                    return Ok(NativeControlOutcome::Unknown);
                }
                let mut writer = lock_recover(&self.writer);
                let Some(writer) = writer.as_mut() else {
                    return Ok(NativeControlOutcome::Unknown);
                };
                if gap.is_zero() {
                    let payload = [pasted.as_slice(), PASTE_END_AND_SUBMIT].concat();
                    if self.write_terminal_input_once(writer, &payload).is_err() {
                        return Ok(NativeControlOutcome::Unknown);
                    }
                } else {
                    // A deferred provider treats a submit key arriving in the same
                    // input burst as pasted text, so the paste and the submit are
                    // separate writes with a bounded settle between them. Submission
                    // is user-initiated and the wait keeps the durable intent,
                    // reservation and authority fences in one critical section.
                    let payload = [pasted.as_slice(), PASTE_END].concat();
                    if self.write_terminal_input_once(writer, &payload).is_err() {
                        return Ok(NativeControlOutcome::Unknown);
                    }
                    std::thread::sleep(gap);
                    if control.verify(&record).is_err() || ready().is_err() {
                        return Ok(NativeControlOutcome::Unknown);
                    }
                    if self.write_terminal_input_once(writer, SUBMIT).is_err() {
                        return Ok(NativeControlOutcome::Unknown);
                    }
                }
                expect_turn = matches!(
                    binding.state,
                    crate::agent::AgentState::WaitingForInput
                        | crate::agent::AgentState::Active
                );
                Ok(control
                    .complete(&record, reservation, Completion::Delivered)
                    .map(outcome_for_receipt)
                    .unwrap_or(NativeControlOutcome::Unknown))
            },
            )
            .map_err(|_| ErrorBody::conflict(SOURCE_CHANGED))?
        };
        drop(_agent);
        let outcome = super::confirm_effect(
            &mut effects, request.terminal_id, since_ms, expect_turn.then_some(true), outcome,
        )
        .await;
        Ok(submit_result(request, outcome))
    }
}

fn submit_result(
    request: &NativeSubmitRequest,
    outcome: NativeControlOutcome,
) -> NativeControlResult {
    NativeControlResult {
        terminal_id: request.terminal_id,
        runtime_epoch: request.runtime_epoch,
        request_id: request.request_id,
        command_digest: request.command_digest,
        outcome,
    }
}
