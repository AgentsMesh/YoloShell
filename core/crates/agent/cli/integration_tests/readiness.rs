use super::fixture::Fixture;
use yolo_agent_cli_readiness::readiness_message;
use yolo_hub_agent_protocol::{
    Activity, AgentAction, AgentSnapshot, Capability, Lifecycle, Prompt, ProtocolError,
    UnavailabilityReason,
};

fn snapshot(activity: Activity, submit_prompt: Capability) -> AgentSnapshot {
    let fixture = Fixture::new();
    let mut fields = fixture
        .agent(activity, Lifecycle::Running {})
        .fields()
        .clone();
    fields.capabilities.submit_prompt = submit_prompt;
    AgentSnapshot::new(fields).unwrap()
}

fn submit() -> AgentAction {
    AgentAction::SubmitPrompt {
        prompt: Prompt::new("hello".to_string()).unwrap(),
    }
}

#[test]
fn a_working_agent_explains_how_to_proceed() {
    let snapshot = snapshot(
        Activity::Working,
        Capability::Unavailable {
            reason: UnavailabilityReason::InputUnavailable,
        },
    );
    assert_eq!(
        readiness_message(&snapshot, submit(), &ProtocolError::UnavailableControl),
        "the agent is working; wait for the turn to finish or interrupt it before sending a prompt"
    );
}

#[test]
fn unsupported_prompt_control_points_at_the_terminal() {
    let snapshot = snapshot(Activity::WaitingForInput, Capability::Unsupported {});
    assert_eq!(
        readiness_message(&snapshot, submit(), &ProtocolError::UnsupportedControl),
        "this agent does not support semantic prompt control; type into its terminal instead"
    );
}

#[test]
fn changed_observations_ask_for_a_retry() {
    let snapshot = snapshot(Activity::WaitingForInput, Capability::Available {});
    assert_eq!(
        readiness_message(&snapshot, submit(), &ProtocolError::StaleObservation),
        "the agent changed since it was observed; retry"
    );
    assert_eq!(
        readiness_message(&snapshot, submit(), &ProtocolError::TargetMismatch),
        "the agent execution changed; select the current agent and retry"
    );
}

#[test]
fn an_unavailable_adapter_is_named() {
    let snapshot = snapshot(
        Activity::WaitingForInput,
        Capability::Unavailable {
            reason: UnavailabilityReason::AdapterUnavailable,
        },
    );
    assert_eq!(
        readiness_message(&snapshot, submit(), &ProtocolError::UnavailableControl),
        "the provider adapter is unavailable; retry after it recovers"
    );
}
