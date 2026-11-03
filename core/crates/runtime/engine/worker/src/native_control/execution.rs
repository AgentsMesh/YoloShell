use super::{
    NativeControlOutcome, NativeControlResult, NativeInterruptRequest, constants::*, outcome_for_receipt,
    observation::journal_error, source,
};
use crate::protocol::ErrorBody;
use crate::worker::{
    AuthenticatedController, WorkerState, agent_monitor_publication_fence, lock_recover, process,
};
use std::sync::Arc;
use yolo_agent_control_journal::{Completion, ReservationResult};

impl WorkerState {
    pub(in crate::worker) async fn interrupt_native_agent(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        request: &NativeInterruptRequest,
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
        let mut expect_end = false;
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
        let root = self.pty_root_identity();
        let group_id = self
            .foreground_process_group()
            .map_err(|_| ErrorBody::conflict(SOURCE_CHANGED))?;
        let group = super::super::run_process_query(move || {
            process::PtyProcessScope::capture(root)?.foreground_group(group_id)
        })
        .await
        .map_err(|_| ErrorBody::conflict(SOURCE_CHANGED))?;
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
            |binding, witness, verify_source| {
                fence.verify()?;
                verify_source()?;
                let delivery = self
                    .provider_runtime
                    .get(&binding.kind)
                    .map_or(yolo_agent_providers::InterruptDelivery::Unsupported, |provider| {
                        provider.interrupt_delivery()
                    });
                self.authorize_control(
                    controller,
                    request.generation,
                    &request.writer_capability,
                )?;
                anyhow::ensure!(self.foreground_process_group()? == group_id, SOURCE_CHANGED);
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
                // No await follows durable intent. Recheck expiry after fsync before
                // the one kernel operation; losing this proof cannot authorize retry.
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
                    .is_ok()
                    && self
                        .foreground_process_group()
                        .is_ok_and(|id| id == group_id);
                let delivered = if admitted {
                    match delivery {
                        // A key-delivered interrupt is what an operator types; the
                        // provider's own reader cancels the turn and keeps running.
                        yolo_agent_providers::InterruptDelivery::Key(key) => {
                            let mut writer = lock_recover(&self.writer);
                            match writer.as_mut() {
                                Some(writer) => self
                                    .write_terminal_input_once(writer, key)
                                    .map(|()| true)
                                    .map_err(Into::into),
                                None => Ok(false),
                            }
                        }
                        yolo_agent_providers::InterruptDelivery::Signal => {
                            #[cfg(test)]
                            self.native_control_test_hooks.run_before_signal();
                            let target = process::ProcessIdentity {
                                pid: witness.fields().process_pid,
                                start_time: witness.fields().process_start_time,
                            };
                            process::signal_process_group_member(group, target, libc::SIGINT)
                        }
                        yolo_agent_providers::InterruptDelivery::Unsupported => Ok(false),
                    }
                } else {
                    Ok(false)
                };
                if matches!(&delivered, Ok(true))
                    && binding.state == crate::agent::AgentState::Running
                {
                    expect_end = true;
                }
                match delivered {
                    Ok(delivered) => {
                        let completion = if delivered {
                            Completion::Delivered
                        } else {
                            Completion::NotDelivered
                        };
                        Ok(control
                            .complete(&record, reservation, completion)
                            .map(outcome_for_receipt)
                            .unwrap_or(NativeControlOutcome::Unknown))
                    }
                    Err(_) => Ok(NativeControlOutcome::Unknown),
                }
            },
            )
            .map_err(|_| ErrorBody::conflict(SOURCE_CHANGED))?
        };
        drop(_agent);
        let outcome = super::confirm_effect(
            &mut effects, request.terminal_id, since_ms, expect_end.then_some(false), outcome,
        )
        .await;
        Ok(result(request, outcome))
    }
}

fn result(request: &NativeInterruptRequest, outcome: NativeControlOutcome) -> NativeControlResult {
    NativeControlResult {
        terminal_id: request.terminal_id,
        runtime_epoch: request.runtime_epoch,
        request_id: request.request_id,
        command_digest: request.command_digest,
        outcome,
    }
}
