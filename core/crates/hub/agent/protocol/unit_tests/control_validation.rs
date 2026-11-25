use uuid::Uuid;
use yolo_hub_agent_protocol::*;

use crate::constants::*;
use crate::fixtures::*;

#[test]
fn control_schema_identity_and_execution_preconditions_fail_closed() {
    let current = AgentSnapshot::new(fields()).unwrap();
    let base = request_fields(&current);
    let mut invalid = base.clone();
    invalid.schema_version += SEQUENCE_STEP as u16;
    assert_eq!(
        AgentControlRequest::new(invalid),
        Err(ProtocolError::UnsupportedSchema)
    );
    let mut invalid = base.clone();
    invalid.request_id = Uuid::nil();
    assert_eq!(
        AgentControlRequest::new(invalid),
        Err(ProtocolError::InvalidRequest)
    );
    let mut invalid = base.clone();
    invalid.target = target();
    assert_eq!(
        AgentControlRequest::new(invalid),
        Err(ProtocolError::TargetMismatch)
    );
    let mut wire = serde_json::to_value(AgentControlRequest::new(base).unwrap()).unwrap();
    wire[REQUEST_ID_FIELD] = Uuid::nil().to_string().into();
    assert!(serde_json::from_value::<AgentControlRequest>(wire).is_err());
}

#[test]
fn response_reference_must_be_nonnil_and_match_agent_execution() {
    let current = waiting(InteractionKind::Approval);
    for mutate in [
        (|reference: &mut InteractionReference| reference.id = Uuid::nil())
            as fn(&mut InteractionReference),
        |reference: &mut InteractionReference| reference.target = target(),
    ] {
        let mut request = response_fields(&current);
        if let AgentAction::Respond { interaction, .. } = &mut request.action {
            mutate(interaction);
        }
        assert_eq!(
            AgentControlRequest::new(request),
            Err(ProtocolError::InvalidInteraction)
        );
    }
}

#[test]
fn response_payloads_are_bounded_and_never_accept_nul_input() {
    let current = waiting(InteractionKind::Approval);
    for invalid in [
        String::new(),
        INVALID_PROVIDER.into(),
        TEXT_CHARACTER.repeat(MAX_CHOICE_ID_BYTES + SEQUENCE_STEP as usize),
    ] {
        let mut fields = response_fields(&current);
        if let AgentAction::Respond { response, .. } = &mut fields.action {
            *response = InteractionResponse::Choice { choice: invalid };
        }
        assert_eq!(
            AgentControlRequest::new(fields),
            Err(ProtocolError::InvalidResponse)
        );
    }
    let text = waiting(InteractionKind::Text);
    for (value, valid) in [
        (String::new(), true),
        (TEXT_RESPONSE.into(), true),
        (TEXT_CHARACTER.repeat(MAX_INTERACTION_TEXT_BYTES), true),
        (
            TEXT_CHARACTER.repeat(MAX_INTERACTION_TEXT_BYTES + SEQUENCE_STEP as usize),
            false,
        ),
        ("nul\0byte".into(), false),
    ] {
        let mut fields = response_fields(&text);
        if let AgentAction::Respond { response, .. } = &mut fields.action {
            *response = InteractionResponse::Text { text: value };
        }
        assert_eq!(AgentControlRequest::new(fields).is_ok(), valid);
    }
}
