use crate::fixtures::*;
use yolo_hub_agent_protocol::{AgentAction, AgentControlRequest, MAX_PROMPT_BYTES, Prompt};
use yolo_hub_relay_control_protocol::*;

const PROMPT: &str = "hello\n世界";
const ESCAPED_CHARACTER: &str = "\\";

fn submit(text: String) -> Request {
    let mut fields = fields();
    let mut control = fields.request.fields().clone();
    control.action = AgentAction::SubmitPrompt {
        prompt: Prompt::new(text).unwrap(),
    };
    fields.request = AgentControlRequest::new(control).unwrap();
    Request::new(fields).unwrap()
}

#[test]
fn prompt_and_maximum_escaped_payload_round_trip_in_both_directions() {
    for text in [PROMPT.into(), ESCAPED_CHARACTER.repeat(MAX_PROMPT_BYTES)] {
        let request = submit(text);
        let encoded = encode_request(&request).unwrap();
        assert!(encoded.len() <= MAX_FRAME_BYTES);
        assert_eq!(decode_request(&encoded).unwrap(), request);
        let reply = response(&request);
        let encoded = encode_response(&reply).unwrap();
        assert!(encoded.len() <= MAX_FRAME_BYTES);
        let decoded = decode_response(&encoded).unwrap();
        assert_eq!(decoded, reply);
        decoded.validate_for(&request).unwrap();
    }
}

#[test]
fn echoed_request_cannot_change_prompt_text_under_the_same_operation_id() {
    let original = submit(PROMPT.into());
    let mut changed = original.fields().clone();
    let mut control = changed.request.fields().clone();
    control.action = AgentAction::SubmitPrompt {
        prompt: Prompt::new(PRIVATE_PAYLOAD.into()).unwrap(),
    };
    changed.request = AgentControlRequest::new(control).unwrap();
    let changed = Request::new(changed).unwrap();
    assert!(response(&changed).validate_for(&original).is_err());
    assert!(response(&original).validate_for(&changed).is_err());
}
