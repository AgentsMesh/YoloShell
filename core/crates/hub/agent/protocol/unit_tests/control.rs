use uuid::Uuid;
use yolo_hub_agent_protocol::*;

use crate::constants::*;
use crate::fixtures::*;

#[test]
fn interrupt_and_scoped_responses_use_the_same_origin_revision() {
    let current = AgentSnapshot::new(fields()).unwrap();
    let request = AgentControlRequest::new(request_fields(&current)).unwrap();
    assert_eq!(request.validate_against(&current), Ok(()));
    assert_eq!(round_trip(&request), request);
    for kind in [
        InteractionKind::Approval,
        InteractionKind::Choice,
        InteractionKind::Text,
    ] {
        let current = waiting(kind);
        let fields = response_fields(&current);
        let request = AgentControlRequest::new(fields.clone()).unwrap();
        assert_eq!(request.fields(), &fields);
        assert_eq!(request.validate_against(&current), Ok(()));
        assert_eq!(round_trip(&request), request);
    }
}

#[test]
fn controls_reject_old_execution_cursor_stale_and_unreachable_state() {
    let current = AgentSnapshot::new(fields()).unwrap();
    let request = AgentControlRequest::new(request_fields(&current)).unwrap();
    assert_eq!(
        request.validate_against(&AgentSnapshot::new(fields()).unwrap()),
        Err(ProtocolError::TargetMismatch)
    );
    let mut newer = current.fields().clone();
    let cursor = newer.cursor;
    newer.cursor = ObservationCursor::new(
        cursor.target(),
        cursor.stream(),
        cursor.sequence() + SEQUENCE_STEP,
    )
    .unwrap();
    assert_eq!(
        request.validate_against(&AgentSnapshot::new(newer).unwrap()),
        Err(ProtocolError::RevisionMismatch)
    );
    let mut stale = current.fields().clone();
    stale.freshness = Freshness::Stale;
    assert_eq!(
        request.validate_against(&AgentSnapshot::new(stale).unwrap()),
        Err(ProtocolError::StaleObservation)
    );
    let mut offline = current.fields().clone();
    offline.reachability = Reachability::Unreachable;
    assert_eq!(
        request.validate_against(&AgentSnapshot::new(offline).unwrap()),
        Err(ProtocolError::Unreachable)
    );
}

#[test]
fn adapter_capability_is_checked_for_each_semantic_operation() {
    let current = waiting(InteractionKind::Approval);
    for action in [request_fields(&current), response_fields(&current)] {
        let request = AgentControlRequest::new(action).unwrap();
        for (capability, error) in [
            (
                Capability::Unsupported {},
                ProtocolError::UnsupportedControl,
            ),
            (
                Capability::Unavailable {
                    reason: UnavailabilityReason::AdapterUnavailable,
                },
                ProtocolError::UnavailableControl,
            ),
        ] {
            let mut changed = current.fields().clone();
            changed.capabilities.interrupt = capability;
            changed.capabilities.respond_interaction = capability;
            assert_eq!(
                request.validate_against(&AgentSnapshot::new(changed).unwrap()),
                Err(error)
            );
        }
    }
}

#[test]
fn responses_cannot_answer_a_replaced_interaction_or_unadvertised_choice() {
    let current = waiting(InteractionKind::Approval);
    let mut fields = response_fields(&current);
    if let AgentAction::Respond { interaction, .. } = &mut fields.action {
        interaction.id = Uuid::new_v4();
    }
    let request = AgentControlRequest::new(fields).unwrap();
    assert_eq!(
        request.validate_against(&current),
        Err(ProtocolError::UnknownInteraction)
    );
    for response in [
        InteractionResponse::Choice {
            choice: ALTERNATE_CHOICE_ID.into(),
        },
        InteractionResponse::Text {
            text: TEXT_RESPONSE.into(),
        },
    ] {
        let mut fields = response_fields(&current);
        if let AgentAction::Respond {
            response: field, ..
        } = &mut fields.action
        {
            *field = response;
        }
        let request = AgentControlRequest::new(fields).unwrap();
        assert_eq!(
            request.validate_against(&current),
            Err(ProtocolError::InvalidResponse)
        );
    }
    let text = waiting(InteractionKind::Text);
    let mut fields = response_fields(&text);
    if let AgentAction::Respond { response, .. } = &mut fields.action {
        *response = InteractionResponse::Choice {
            choice: CHOICE_ID.into(),
        };
    }
    assert_eq!(
        AgentControlRequest::new(fields)
            .unwrap()
            .validate_against(&text),
        Err(ProtocolError::InvalidResponse)
    );
}
