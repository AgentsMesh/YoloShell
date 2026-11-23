use yolo_hub_agent_protocol::{
    Activity, AgentAction, AgentSnapshot, AgentSnapshotFields, Capability, ProtocolError,
    UnavailabilityReason,
};

/// Errors from the local readiness check name the next action instead of the
/// protocol term: the common case is a prompt sent while the agent works.
pub fn readiness_message(
    snapshot: &AgentSnapshot,
    action: AgentAction,
    error: &ProtocolError,
) -> String {
    match error {
        ProtocolError::UnsupportedControl => match action {
            AgentAction::SubmitPrompt { .. } => {
                "this agent does not support semantic prompt control; type into its terminal instead"
            }
            AgentAction::Interrupt {} => "this agent does not support semantic interrupt",
            AgentAction::Respond { .. } => "this agent does not support interaction responses",
        }
        .into(),
        ProtocolError::UnavailableControl => unavailable_control(snapshot.fields(), &action).into(),
        ProtocolError::StaleObservation | ProtocolError::RevisionMismatch => {
            "the agent changed since it was observed; retry".into()
        }
        ProtocolError::TargetMismatch => {
            "the agent execution changed; select the current agent and retry".into()
        }
        ProtocolError::Unreachable => "the agent is unreachable; check its terminal".into(),
        other => format!("the agent cannot accept this control ({other})"),
    }
}

fn unavailable_control(fields: &AgentSnapshotFields, action: &AgentAction) -> &'static str {
    match action {
        AgentAction::SubmitPrompt { .. } => match fields.capabilities.submit_prompt {
            Capability::Unavailable { reason } => match reason {
                UnavailabilityReason::InputUnavailable
                    if matches!(
                        fields.activity,
                        Activity::Working | Activity::WaitingForApproval
                    ) =>
                {
                    "the agent is working; wait for the turn to finish or interrupt it before sending a prompt"
                }
                UnavailabilityReason::InputUnavailable => {
                    "the agent is not ready for input right now; retry shortly"
                }
                UnavailabilityReason::NotRunning => "the agent is not running",
                UnavailabilityReason::AdapterUnavailable => {
                    "the provider adapter is unavailable; retry after it recovers"
                }
                UnavailabilityReason::StateUnknown => {
                    "the agent state is unknown; retry after it refreshes"
                }
                UnavailabilityReason::NoPendingInteraction => "the agent has no pending interaction",
            },
            _ => "the agent cannot accept a prompt right now; retry shortly",
        },
        AgentAction::Interrupt {} => "the agent cannot be interrupted right now",
        AgentAction::Respond { .. } => "the interaction cannot be answered right now",
    }
}
