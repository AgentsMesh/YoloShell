use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    CommandOutcome, CommandReceipt, FIRST_OBSERVATION_SEQUENCE, ObservationCursor,
};
use yolo_hub_identity::{AgentAddress, AgentTarget, RuntimeTarget, TerminalAddress};
use yolo_hub_relay_control_protocol::{Outcome, Request, RequestFields, Response, SCHEMA_VERSION};

#[path = "constants.rs"]
mod constants;
pub use constants::*;

pub fn fields() -> RequestFields {
    let target = AgentTarget {
        address: AgentAddress {
            hub_id: HUB_ID.parse().unwrap(),
            agent_id: AGENT_ID.parse().unwrap(),
        },
        execution_id: EXECUTION_ID.parse().unwrap(),
    };
    RequestFields {
        schema_version: SCHEMA_VERSION,
        launch_id: LAUNCH_ID.parse().unwrap(),
        terminal: RuntimeTarget {
            address: TerminalAddress {
                hub_id: target.address.hub_id,
                terminal_id: SESSION_ID.parse().unwrap(),
            },
            runtime_epoch: RUNTIME_EPOCH.parse().unwrap(),
        },
        request: AgentControlRequest::new(AgentControlFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            request_id: REQUEST_ID.parse().unwrap(),
            target,
            expected_cursor: ObservationCursor::new(
                target,
                STREAM_ID.parse().unwrap(),
                FIRST_OBSERVATION_SEQUENCE,
            )
            .unwrap(),
            action: AgentAction::Interrupt {},
        })
        .unwrap(),
    }
}

pub fn request() -> Request {
    Request::new(fields()).unwrap()
}

pub fn receipt(request: &Request, outcome: CommandOutcome) -> CommandReceipt {
    let fields = request.fields().request.fields();
    let mut receipt = CommandReceipt::accepted(fields.request_id, fields.target).unwrap();
    receipt.advance(outcome).unwrap();
    receipt
}

pub fn response(request: &Request) -> Response {
    Response::new(
        request,
        Outcome::Receipt {
            receipt: receipt(request, CommandOutcome::Delivered {}),
        },
    )
    .unwrap()
}

pub fn alternate_uuid() -> Uuid {
    ALTERNATE_ID.parse().unwrap()
}

pub fn modified_json(
    value: &impl serde::Serialize,
    path: &str,
    replacement: serde_json::Value,
) -> Vec<u8> {
    let mut value = serde_json::to_value(value).unwrap();
    *value.pointer_mut(path).expect("fixture path exists") = replacement;
    serde_json::to_vec(&value).unwrap()
}

pub fn extra_field(value: &impl serde::Serialize, path: &str) -> Vec<u8> {
    let mut value = serde_json::to_value(value).unwrap();
    value
        .pointer_mut(path)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(EXTRA_FIELD.into(), EXTRA_VALUE.into());
    serde_json::to_vec(&value).unwrap()
}
