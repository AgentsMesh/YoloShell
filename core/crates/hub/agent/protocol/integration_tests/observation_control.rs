use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::ExecutionId;

use crate::fixtures::*;

#[test]
fn reconnect_and_replaced_execution_do_not_replay_old_agent_controls() {
    let origin = waiting(InteractionKind::Approval);
    let observer: AgentSnapshot = round_trip(&origin);
    let command = AgentControlRequest::new(response_fields(&observer)).unwrap();
    let received: AgentControlRequest = round_trip(&command);
    assert_eq!(received.validate_against(&origin), Ok(()));

    let mut unavailable = origin.fields().clone();
    unavailable.freshness = Freshness::Stale;
    unavailable.reachability = Reachability::Unreachable;
    let cached = AgentSnapshot::new(unavailable).unwrap();
    assert_eq!(cached.fields().lifecycle, Lifecycle::Running {});
    assert_eq!(
        received.validate_against(&cached),
        Err(ProtocolError::StaleObservation)
    );

    let mut replacement = origin.fields().clone();
    let mut target = origin.cursor().target();
    target.execution_id = ExecutionId::new();
    replacement.cursor =
        ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE).unwrap();
    replacement.interaction = Some(interaction(target, InteractionKind::Approval));
    let replacement = AgentSnapshot::new(replacement).unwrap();
    assert_eq!(
        replacement.cursor().position_after(origin.cursor()),
        CursorPosition::ResetRequired
    );
    assert_eq!(
        replacement.cursor().target().address,
        origin.cursor().target().address
    );
    assert_eq!(
        received.validate_against(&replacement),
        Err(ProtocolError::TargetMismatch)
    );
}

#[test]
fn missing_reply_stays_unknown_until_origin_reports_effect_evidence() {
    let current = waiting(InteractionKind::Text);
    let command = AgentControlRequest::new(response_fields(&current)).unwrap();
    command.validate_against(&current).unwrap();
    let mut receipt =
        CommandReceipt::accepted(command.fields().request_id, command.fields().target).unwrap();
    receipt.advance(CommandOutcome::Delivered {}).unwrap();
    receipt.advance(CommandOutcome::Unknown {}).unwrap();
    let mut reconnected: CommandReceipt = round_trip(&receipt);
    assert_eq!(reconnected.request_id(), command.fields().request_id);
    assert_eq!(
        reconnected.advance(CommandOutcome::Accepted {}),
        Err(ProtocolError::InvalidTransition)
    );
    reconnected.advance(CommandOutcome::Confirmed {}).unwrap();
    assert_eq!(
        reconnected.advance(CommandOutcome::Unknown {}),
        Err(ProtocolError::InvalidTransition)
    );
}

#[test]
fn hook_observation_does_not_implicitly_grant_semantic_approval_capability() {
    let mut fields = waiting(InteractionKind::Approval).fields().clone();
    fields.capabilities.respond_interaction = Capability::Unsupported {};
    let current = AgentSnapshot::new(fields).unwrap();
    assert_eq!(current.fields().source, ObservationSource::ProviderHook);
    let command = AgentControlRequest::new(response_fields(&current)).unwrap();
    assert_eq!(
        command.validate_against(&current),
        Err(ProtocolError::UnsupportedControl)
    );
}
