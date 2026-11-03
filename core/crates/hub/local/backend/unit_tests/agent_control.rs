use super::fixtures::agent_control::*;
use crate::agent_control::reject_observation_only_control;
use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::{AgentId, ExecutionId, RuntimeEpoch, TerminalId};
use yolo_hub_terminal_protocol::LeaseGrant;

#[test]
fn native_rejection_is_definitive_and_correlated_without_changing_the_observation() {
    let observed = snapshot();
    let request = request(&observed);
    let grant = authority();
    let receipt = reject_observation_only_control(&request, grant, &observed);
    assert_eq!(receipt.request_id(), request.fields().request_id);
    assert_eq!(receipt.target(), request.fields().target);
    assert_eq!(
        receipt.outcome(),
        CommandOutcome::Failed {
            code: FailureCode::Unsupported
        }
    );
    assert_eq!(
        observed.fields().capabilities,
        AgentCapabilities::UNSUPPORTED
    );
    assert_eq!(observed.fields().lifecycle, Lifecycle::Running {});
}

#[test]
fn provider_capability_cannot_invent_a_native_effect_adapter() {
    let mut fields = snapshot().fields().clone();
    fields.capabilities.interrupt = Capability::Available {};
    let observed = AgentSnapshot::new(fields).unwrap();
    let request = request(&observed);
    assert!(request.validate_against(&observed).is_ok());
    assert_eq!(
        reject_observation_only_control(&request, authority(), &observed).outcome(),
        CommandOutcome::Failed {
            code: FailureCode::Unsupported
        }
    );
}

#[test]
fn changed_observation_or_execution_is_not_reported_as_unsupported_success() {
    let original = snapshot();
    let request = request(&original);
    let mutations: &[fn(&mut AgentSnapshotFields)] = &[
        |fields| fields.freshness = Freshness::Stale,
        |fields| fields.reachability = Reachability::Unreachable,
        |fields| {
            fields.cursor = ObservationCursor::new(
                fields.cursor.target(),
                Uuid::new_v4(),
                FIRST_OBSERVATION_SEQUENCE,
            )
            .unwrap()
        },
        |fields| {
            fields.cursor = ObservationCursor::new(
                fields.cursor.target(),
                fields.cursor.stream(),
                fields.cursor.sequence() + FIRST_OBSERVATION_SEQUENCE,
            )
            .unwrap()
        },
        |fields| {
            let mut target = fields.cursor.target();
            target.execution_id = ExecutionId::new();
            fields.cursor =
                ObservationCursor::new(target, fields.cursor.stream(), fields.cursor.sequence())
                    .unwrap();
        },
        |fields| {
            let mut target = fields.cursor.target();
            target.address.agent_id = AgentId::new();
            fields.cursor =
                ObservationCursor::new(target, fields.cursor.stream(), fields.cursor.sequence())
                    .unwrap();
        },
    ];
    for mutate in mutations {
        let mut fields = original.fields().clone();
        mutate(&mut fields);
        let observed = AgentSnapshot::new(fields).unwrap();
        assert_eq!(
            reject_observation_only_control(&request, authority(), &observed).outcome(),
            CommandOutcome::Failed {
                code: FailureCode::PreconditionsChanged
            }
        );
    }
}

#[test]
fn runtime_and_session_must_match_before_considering_provider_support() {
    let observed = snapshot();
    let request = request(&observed);
    for session_changed in [false, true] {
        let mut fields = *authority().fields();
        if session_changed {
            fields.target.address.terminal_id = TerminalId::new();
        } else {
            fields.target.runtime_epoch = RuntimeEpoch::new();
        }
        let grant = LeaseGrant::new(fields).unwrap();
        assert_eq!(
            reject_observation_only_control(&request, grant, &observed).outcome(),
            CommandOutcome::Failed {
                code: FailureCode::PreconditionsChanged
            }
        );
    }
}

#[test]
fn unavailable_capability_is_a_changed_precondition() {
    let mut fields = snapshot().fields().clone();
    fields.capabilities.interrupt = Capability::Unavailable {
        reason: UnavailabilityReason::AdapterUnavailable,
    };
    let observed = AgentSnapshot::new(fields).unwrap();
    assert_eq!(
        reject_observation_only_control(&request(&observed), authority(), &observed).outcome(),
        CommandOutcome::Failed {
            code: FailureCode::PreconditionsChanged
        }
    );
}

#[tokio::test]
async fn backend_without_agent_observations_neither_advertises_nor_dispatches_controls() {
    use super::fixtures::constants::ABSENT_HOST_SOCKET;
    use crate::LocalBackend;
    use yolo_hub_rpc_protocol::{Command, Method};
    use yolo_hub_terminal_protocol::PublicError;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap();
    let socket = path.join(ABSENT_HOST_SOCKET);
    let observed = snapshot();
    let hub = observed.cursor().target().address.hub_id;
    let backend = LocalBackend::new(hub, &path, &socket).unwrap();
    assert!(!backend.supports(Method::AgentControl));
    assert!(backend.supports(Method::ListHubs));
    let result = backend
        .execute(&Command::AgentControl {
            request: request(&observed),
            grant: authority(),
        })
        .await;
    assert_eq!(result, Err(PublicError::Unsupported));
    assert!(!socket.exists());
}
