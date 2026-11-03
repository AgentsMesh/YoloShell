use crate::fixtures::*;
use yolo_hub_agent_protocol::*;

const PROMPT: &str = "Explain this implementation\n保留上下文";
const PROMPT_FIELD: &str = "prompt";
const ESCAPED_PROMPT: &str = "\x1b[201~exit\r";

fn submit_fields(snapshot: &AgentSnapshot) -> AgentControlFields {
    let mut request = request_fields(snapshot);
    request.action = AgentAction::SubmitPrompt {
        prompt: Prompt::new(PROMPT.into()).unwrap(),
    };
    request
}

#[test]
fn prompt_submission_is_separate_from_answering_a_pending_interaction() {
    let mut observed = fields();
    observed.interaction = None;
    observed.capabilities.submit_prompt = Capability::Available {};
    let current = AgentSnapshot::new(observed).unwrap();
    let request = AgentControlRequest::new(submit_fields(&current)).unwrap();
    assert_eq!(request.validate_against(&current), Ok(()));
    assert_eq!(round_trip(&request), request);
    for (capability, expected) in [
        (
            Capability::Unsupported {},
            ProtocolError::UnsupportedControl,
        ),
        (
            Capability::Unavailable {
                reason: UnavailabilityReason::InputUnavailable,
            },
            ProtocolError::UnavailableControl,
        ),
    ] {
        let mut changed = current.fields().clone();
        changed.capabilities.submit_prompt = capability;
        assert_eq!(
            request.validate_against(&AgentSnapshot::new(changed).unwrap()),
            Err(expected)
        );
    }
}

#[test]
fn prompt_wire_rejects_terminal_injection_and_missing_payload() {
    let current = AgentSnapshot::new(fields()).unwrap();
    let request = AgentControlRequest::new(submit_fields(&current)).unwrap();
    let mut malformed = serde_json::to_value(request).unwrap();
    malformed[AGENT_ACTION_FIELD][PROMPT_FIELD] = serde_json::json!(ESCAPED_PROMPT);
    assert!(serde_json::from_value::<AgentControlRequest>(malformed.clone()).is_err());
    malformed[AGENT_ACTION_FIELD]
        .as_object_mut()
        .unwrap()
        .remove(PROMPT_FIELD);
    assert!(serde_json::from_value::<AgentControlRequest>(malformed).is_err());
}
