use crate::{Chat, SendPhase};
use yolo_hub_agent_protocol::{AgentSnapshot, Capability, Freshness, Reachability};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputReadiness {
    Ready,
    AwaitingObservation,
    Unavailable,
}

impl Chat {
    /// Delivery consumes the observed precondition; it does not confirm a new idle editor.
    pub fn input_readiness(&self, snapshot: &AgentSnapshot) -> InputReadiness {
        let fields = snapshot.fields();
        if self.retired
            || self.mode != crate::Mode::Chat
            || fields.cursor.target() != self.binding.agent
            || fields.terminal != self.binding.runtime
        {
            return InputReadiness::Unavailable;
        }
        let consumed = matches!(
            self.composer.phase(),
            Some(SendPhase::Delivered | SendPhase::Confirmed)
        );
        if consumed
            && self
                .composer
                .request()
                .is_some_and(|request| request.fields().expected_cursor == fields.cursor)
        {
            return InputReadiness::AwaitingObservation;
        }
        if self.composer.locked()
            || fields.freshness != Freshness::Current
            || fields.reachability != Reachability::Reachable
            || fields.capabilities.submit_prompt != (Capability::Available {})
        {
            InputReadiness::Unavailable
        } else {
            InputReadiness::Ready
        }
    }
}
