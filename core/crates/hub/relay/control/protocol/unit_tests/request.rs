use crate::fixtures::*;
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AgentAction, AgentControlRequest, InteractionReference, InteractionResponse,
};
use yolo_hub_relay_control_protocol::{ProtocolError, Request, RequestFields};

#[test]
fn interrupt_preserves_exact_semantic_and_terminal_identity() {
    let fields = fields();
    let request = Request::new(fields.clone()).unwrap();
    assert_eq!(request.fields(), &fields);
    assert_eq!(request.validate(), Ok(()));
    assert_eq!(RequestFields::from(request), fields);
}

#[test]
fn rejects_unsupported_schema_nil_launch_and_foreign_origin() {
    let mut fields = fields();
    fields.schema_version = UNSUPPORTED_SCHEMA;
    assert_eq!(
        Request::new(fields.clone()),
        Err(ProtocolError::UnsupportedSchema)
    );
    fields = crate::fixtures::fields();
    fields.launch_id = Uuid::nil();
    assert_eq!(
        Request::new(fields.clone()),
        Err(ProtocolError::InvalidIdentity)
    );
    fields = crate::fixtures::fields();
    fields.terminal.address.hub_id = ALTERNATE_ID.parse().unwrap();
    assert_eq!(Request::new(fields), Err(ProtocolError::OriginMismatch));
}

#[test]
fn rejects_other_public_actions_even_when_they_are_valid() {
    let mut fields = fields();
    let mut control = fields.request.fields().clone();
    control.action = AgentAction::Respond {
        interaction: InteractionReference {
            target: control.target,
            id: alternate_uuid(),
        },
        response: InteractionResponse::Text {
            text: PRIVATE_PAYLOAD.into(),
        },
    };
    fields.request = AgentControlRequest::new(control).unwrap();
    assert_eq!(Request::new(fields), Err(ProtocolError::UnsupportedAction));
}

#[test]
fn typed_construction_cannot_bypass_validation() {
    let mut fields = fields();
    fields.schema_version = UNSUPPORTED_SCHEMA;
    assert_eq!(Request::new(fields), Err(ProtocolError::UnsupportedSchema));
}
