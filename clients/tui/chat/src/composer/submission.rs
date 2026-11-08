use super::{Composer, DraftError, SendPhase, Submission};
use crate::{Chat, Mode};
use uuid::Uuid;
use yolo_agent_input::Prompt;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    AgentSnapshot, CommandOutcome, CommandReceipt, ObservationCursor,
};

impl Chat {
    /// Caller supplies writer authority separately when dispatching the public AgentControl.
    pub fn submit(&mut self, snapshot: &AgentSnapshot) -> Result<AgentControlRequest, DraftError> {
        if self.retired
            || self.mode != Mode::Chat
            || snapshot.cursor().target() != self.binding.agent
            || snapshot.fields().terminal != self.binding.runtime
        {
            return Err(DraftError::WrongTarget);
        }
        self.composer.editable()?;
        if self.input_readiness(snapshot) == crate::InputReadiness::AwaitingObservation {
            return Err(DraftError::ObservationPending);
        }
        let prompt = Prompt::new(self.composer.text.clone()).map_err(|_| DraftError::Empty)?;
        let request = AgentControlRequest::new(AgentControlFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            request_id: Uuid::new_v4(),
            target: self.binding.agent,
            expected_cursor: snapshot.cursor(),
            action: AgentAction::SubmitPrompt { prompt },
        })
        .map_err(|_| DraftError::Unavailable)?;
        request
            .validate_against(snapshot)
            .map_err(|_| DraftError::Unavailable)?;
        let receipt = CommandReceipt::accepted(request.fields().request_id, self.binding.agent)
            .map_err(|_| DraftError::Unavailable)?;
        self.composer.submission = Some(Submission {
            request: request.clone(),
            receipt,
            phase: SendPhase::Sending,
            awaiting_dispatch_result: true,
        });
        Ok(request)
    }
}

impl Composer {
    pub fn received(&mut self, incoming: CommandReceipt) -> bool {
        let Some(current) = &mut self.submission else {
            return false;
        };
        if incoming.request_id() != current.receipt.request_id()
            || incoming.target() != current.receipt.target()
            || current.phase == SendPhase::NotSent
        {
            return false;
        }
        let outcome = incoming.outcome();
        if matches!(current.phase, SendPhase::Delivered | SendPhase::Confirmed)
            && matches!(
                outcome,
                CommandOutcome::Accepted {} | CommandOutcome::Unknown {}
            )
        {
            return false;
        }
        let was_pending = current.phase.blocks();
        if current.receipt.advance(outcome).is_err() {
            return false;
        }
        current.awaiting_dispatch_result = false;
        current.phase = match outcome {
            CommandOutcome::Accepted {} => SendPhase::Accepted,
            CommandOutcome::Delivered {} => SendPhase::Delivered,
            CommandOutcome::Confirmed {} => SendPhase::Confirmed,
            CommandOutcome::Unknown {} => SendPhase::Unknown,
            CommandOutcome::Failed { code } => SendPhase::Rejected(code),
        };
        if was_pending
            && matches!(
                outcome,
                CommandOutcome::Delivered {} | CommandOutcome::Confirmed {}
            )
        {
            self.text.clear();
            self.cursor = 0;
        }
        true
    }
    /// A prompt receipt has no later public result after turn start or delivery.
    /// Once a newer observation exists, keeping this submission would report a
    /// wait that is already over and keep an Accepted receipt locked forever.
    pub fn observe(&mut self, cursor: ObservationCursor) -> bool {
        let Some(current) = &self.submission else {
            return false;
        };
        if !matches!(
            current.phase,
            SendPhase::Accepted | SendPhase::Delivered | SendPhase::Confirmed
        ) || current.request.fields().expected_cursor == cursor
        {
            return false;
        }
        self.submission = None;
        true
    }
    pub fn not_sent(&mut self, request: Uuid) -> bool {
        let Some(current) = &mut self.submission else {
            return false;
        };
        if current.request.fields().request_id != request
            || !current.awaiting_dispatch_result
            || !matches!(current.phase, SendPhase::Sending | SendPhase::Unknown)
        {
            return false;
        }
        // A local disconnect may precede the queue owner's exact not-dispatched result.
        // A provider receipt, including Unknown or Accepted, has already closed this path.
        current.awaiting_dispatch_result = false;
        current.phase = SendPhase::NotSent;
        true
    }
    pub fn disconnected(&mut self) {
        if let Some(current) = &mut self.submission {
            if matches!(current.phase, SendPhase::Sending | SendPhase::Accepted) {
                let _ = current.receipt.advance(CommandOutcome::Unknown {});
                current.phase = SendPhase::Unknown;
            }
        }
    }
}
