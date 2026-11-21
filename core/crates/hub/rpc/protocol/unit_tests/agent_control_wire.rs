use crate::{
    commands::request,
    fixtures::{constants::*, *},
};
use yolo_hub_agent_protocol::{
    AgentAction, AgentControlRequest, InteractionReference, InteractionResponse,
};
use yolo_hub_rpc_protocol::{
    Command, HUB_PROTOCOL_VERSION, RPC_METHOD_TAG, RPC_REQUEST_ID_FIELD, RPC_VERSION_FIELD, Reply,
    Request, Response,
};
use yolo_hub_terminal_protocol::PublicError;

#[path = "../test_support/agent_control_constants.rs"]
mod constants;
use constants::*;

#[test]
fn control_request_capnp_preserves_grant_and_scoped_interaction_without_ignored_fields() {
    let mut fields = agent_control().fields().clone();
    fields.action = AgentAction::Respond {
        interaction: InteractionReference {
            target: fields.target,
            id: id(CREATION_ID),
        },
        response: InteractionResponse::Choice {
            choice: CHOICE_ID.into(),
        },
    };
    let original = request(Command::AgentControl {
        request: AgentControlRequest::new(fields).unwrap(),
        grant: grant(),
    });
    let wire = serde_json::to_value(&original).unwrap();
    assert_eq!(wire[COMMAND_FIELD][RPC_METHOD_TAG], CONTROL_METHOD);
    assert_eq!(
        serde_json::from_value::<Request>(wire.clone()).unwrap(),
        original
    );
    for path in [
        vec![COMMAND_FIELD],
        vec![COMMAND_FIELD, REQUEST_FIELD],
        vec![COMMAND_FIELD, GRANT_FIELD],
        vec![COMMAND_FIELD, REQUEST_FIELD, TARGET_FIELD],
        vec![COMMAND_FIELD, REQUEST_FIELD, EXPECTED_CURSOR_FIELD],
        vec![COMMAND_FIELD, REQUEST_FIELD, ACTION_FIELD],
        vec![
            COMMAND_FIELD,
            REQUEST_FIELD,
            ACTION_FIELD,
            INTERACTION_FIELD,
        ],
        vec![COMMAND_FIELD, REQUEST_FIELD, ACTION_FIELD, RESPONSE_FIELD],
    ] {
        let mut changed = wire.clone();
        let mut value = &mut changed;
        for field in path {
            value = value.get_mut(field).unwrap();
        }
        value
            .as_object_mut()
            .unwrap()
            .insert(UNKNOWN_FIELD.into(), true.into());
        assert!(serde_json::from_value::<Request>(changed).is_err());
    }
    let mut wrong_schema = wire.clone();
    wrong_schema[COMMAND_FIELD][REQUEST_FIELD][SCHEMA_FIELD] = u16::MAX.into();
    assert!(serde_json::from_value::<Request>(wrong_schema).is_err());
    for field in [REQUEST_FIELD, GRANT_FIELD] {
        let mut missing = wire.clone();
        missing[COMMAND_FIELD]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(serde_json::from_value::<Request>(missing).is_err());
    }
}

#[test]
fn nil_inner_identity_unknown_receipt_fields_and_envelope_downgrades_fail_closed() {
    let request = request(Command::AgentControl {
        request: agent_control(),
        grant: grant(),
    });
    let mut wire = serde_json::to_value(&request).unwrap();
    wire[COMMAND_FIELD][REQUEST_FIELD][RPC_REQUEST_ID_FIELD] = uuid::Uuid::nil().to_string().into();
    assert!(serde_json::from_value::<Request>(wire).is_err());
    let response = Response::new(
        &request,
        hub(HUB_ID),
        Reply::AgentControl {
            receipt: agent_receipt(),
        },
    );
    for path in [
        vec![RECEIPT_FIELD],
        vec![RECEIPT_FIELD, TARGET_FIELD],
        vec![RECEIPT_FIELD, OUTCOME_FIELD],
    ] {
        let mut wire = serde_json::to_value(&response.reply).unwrap();
        let mut value = &mut wire;
        for field in path {
            value = value.get_mut(field).unwrap();
        }
        value
            .as_object_mut()
            .unwrap()
            .insert(UNKNOWN_FIELD.into(), true.into());
        assert!(serde_json::from_value::<Reply>(wire).is_err());
    }
    assert!(HUB_PROTOCOL_VERSION > PREVIOUS_RPC_VERSION);
    let mut old_request = serde_json::to_value(&request).unwrap();
    old_request[RPC_VERSION_FIELD] = PREVIOUS_RPC_VERSION.into();
    assert!(serde_json::from_value::<Request>(old_request).is_err());
    let mut old_response = response;
    old_response.version = PREVIOUS_RPC_VERSION;
    assert_eq!(
        old_response.validate_for(&request),
        Err(PublicError::UnsupportedSchema)
    );
}
