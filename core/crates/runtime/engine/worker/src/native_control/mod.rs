mod constants;
mod dispatch;
mod execution;
mod generic_hook_source;
pub(crate) mod interaction;
mod observation;
mod prompt_readiness;
mod respond;
mod source;
mod state;
mod submit;
#[cfg(test)]
#[path = "../../test_support/unit/native/native_control_hooks.rs"]
pub(in crate::worker) mod test_hooks;
pub(in crate::worker) use dispatch::dispatch;
pub(in crate::worker) use generic_hook_source::with_control_source as with_generic_hook_control_source;
pub use yolo_runtime_protocol::{
    NativeControlLookupRequest, NativeControlOutcome, NativeControlResult, NativeInterruptRequest,
    NativeObservationRequest, NativeRespondRequest, NativeSubmitRequest,
};
use crate::agent::AgentState;
use crate::worker::WorkerEvent;
use std::time::Duration;
use tokio::sync::broadcast;

const EFFECT_WINDOW: Duration = Duration::from_millis(2_000);

impl crate::worker::WorkerState {
    /// Controller generation and writer lease are one admission fact for every
    /// native control; callers add their operation-specific source checks.
    pub(super) fn authorize_control(
        &self,
        controller: crate::worker::AuthenticatedController,
        generation: u64,
        writer_capability: &crate::model::WriterCapability,
    ) -> anyhow::Result<()> {
        self.validate_controller(controller)
            .map_err(|_| anyhow::anyhow!(constants::SOURCE_CHANGED))?;
        self.validate_lease_admitted(generation, writer_capability)
            .map_err(|_| anyhow::anyhow!(constants::SOURCE_CHANGED))
    }
}

pub(in crate::worker) async fn confirm_effect(
    events: &mut broadcast::Receiver<WorkerEvent>,
    terminal_id: uuid::Uuid,
    since_ms: i64,
    turn_started: Option<bool>,
    outcome: NativeControlOutcome,
) -> NativeControlOutcome {
    let Some(turn_started) = turn_started else { return outcome };
    if cfg!(test) || outcome != NativeControlOutcome::Delivered {
        return outcome;
    }
    let observed = matches!(
        tokio::time::timeout(EFFECT_WINDOW, async {
            loop {
                match events.recv().await {
                    Ok(WorkerEvent::Agent { binding, .. })
                        if binding.terminal_id == terminal_id
                            && binding.updated_at_ms >= since_ms
                            && if turn_started {
                                binding.state == AgentState::Running
                            } else {
                                matches!(binding.state, AgentState::WaitingForInput | AgentState::Exited)
                            } =>
                    {
                        break true;
                    }
                    Ok(_) => {}
                    Err(_) => break false,
                }
            }
        })
        .await,
        Ok(true)
    );
    match (observed, turn_started) {
        (false, _) => outcome,
        (true, true) => NativeControlOutcome::Accepted,
        (true, false) => NativeControlOutcome::Confirmed,
    }
}

pub(crate) fn outcome_for_receipt(receipt: yolo_agent_control_journal::Receipt) -> NativeControlOutcome {
    match receipt {
        yolo_agent_control_journal::Receipt::Unknown => NativeControlOutcome::Unknown,
        yolo_agent_control_journal::Receipt::Delivered => NativeControlOutcome::Delivered,
        yolo_agent_control_journal::Receipt::NotDelivered => NativeControlOutcome::NotDelivered,
    }
}
