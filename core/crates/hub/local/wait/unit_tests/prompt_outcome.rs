use super::observed_outcome;
use yolo_hub_agent_protocol::AgentPromptOutcome;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn post_delivery_observation_errors_change_only_the_turn_outcome() {
    for (failure, expected) in [
        (PublicError::NotFound, AgentPromptOutcome::Exited),
        (PublicError::Unreachable, AgentPromptOutcome::Unknown),
    ] {
        assert_eq!(observed_outcome(Err(failure)), (expected, None));
    }
}
