use crate::{
    commands::request,
    fixtures::{constants::*, *},
};
use yolo_hub_agent_protocol::{
    AgentControlRequest, CommandOutcome, CommandReceipt, FailureCode, ObservationCursor,
};
use yolo_hub_identity::{AgentId, ExecutionId};
use yolo_hub_rpc_protocol::{Command, Method, Reply, Request, Response, SUPPORTED_METHODS};
use yolo_hub_terminal_protocol::{LeaseGrant, PublicError};

#[test]
fn semantic_control_is_correlated_mutating_routable_and_separate_from_file_operations() {
    let control = agent_control();
    let command = Command::AgentControl {
        request: control.clone(),
        grant: grant(),
    };
    assert_eq!(command.method(), Method::AgentControl);
    assert!(SUPPORTED_METHODS.contains(&Method::AgentControl));
    assert!(command.mutates());
    assert!(!command.local_only());
    assert!(!command.requires_read_reply_admission());
    assert_eq!(command.request_id(), Some(control.fields().request_id));
    let envelope = request(command);
    assert_eq!(envelope.fields().request_id, control.fields().request_id);
    let mut changed = envelope.fields().clone();
    changed.request_id = id(SECOND_REQUEST_ID);
    assert_eq!(Request::try_from(changed), Err(PublicError::InvalidRequest));
}

#[test]
fn agent_origin_lease_origin_and_destination_must_all_agree() {
    for agent_origin in [hub(HUB_ID), hub(SECOND_HUB_ID)] {
        for lease_origin in [hub(HUB_ID), hub(SECOND_HUB_ID)] {
            for destination in [hub(HUB_ID), hub(SECOND_HUB_ID)] {
                let mut fields = agent_control().fields().clone();
                fields.target.address.hub_id = agent_origin;
                fields.expected_cursor = ObservationCursor::new(
                    fields.target,
                    fields.expected_cursor.stream(),
                    fields.expected_cursor.sequence(),
                )
                .unwrap();
                let mut lease = grant_fields();
                lease.target.address.hub_id = lease_origin;
                let command = Command::AgentControl {
                    request: AgentControlRequest::new(fields).unwrap(),
                    grant: LeaseGrant::new(lease).unwrap(),
                };
                let valid = agent_origin == lease_origin && agent_origin == destination;
                assert_eq!(
                    command.validate_origin(destination),
                    if valid {
                        Ok(())
                    } else {
                        Err(PublicError::OriginMismatch)
                    }
                );
                assert_eq!(Request::new(destination, command).is_ok(), valid);
            }
        }
    }
}

#[test]
fn same_origin_receipt_cannot_replace_agent_execution_or_inner_request_id() {
    let control = agent_control();
    let envelope = request(Command::AgentControl {
        request: control.clone(),
        grant: grant(),
    });
    let exact = control.fields().target;
    let mut other_agent = exact;
    other_agent.address.agent_id = AgentId::new();
    let mut other_execution = exact;
    other_execution.execution_id = ExecutionId::new();
    let mut other_origin = exact;
    other_origin.address.hub_id = hub(SECOND_HUB_ID);
    for (target, request_id) in [
        (other_agent, control.fields().request_id),
        (other_execution, control.fields().request_id),
        (other_origin, control.fields().request_id),
        (exact, id(SECOND_REQUEST_ID)),
    ] {
        let receipt = CommandReceipt::accepted(request_id, target).unwrap();
        let response = Response::new(&envelope, hub(HUB_ID), Reply::AgentControl { receipt });
        assert_eq!(
            response.validate_for(&envelope),
            Err(PublicError::InvalidPayload)
        );
    }
}

#[test]
fn receipts_preserve_delivery_classification_and_do_not_imply_provider_support() {
    let control = agent_control();
    let envelope = request(Command::AgentControl {
        request: control.clone(),
        grant: grant(),
    });
    for outcome in [
        CommandOutcome::Accepted {},
        CommandOutcome::Delivered {},
        CommandOutcome::Confirmed {},
        CommandOutcome::Unknown {},
        CommandOutcome::Failed {
            code: FailureCode::Unsupported,
        },
    ] {
        let mut receipt =
            CommandReceipt::accepted(control.fields().request_id, control.fields().target).unwrap();
        receipt.advance(outcome).unwrap();
        let response = Response::new(&envelope, hub(HUB_ID), Reply::AgentControl { receipt });
        assert_eq!(response.validate_for(&envelope), Ok(()));
    }
    assert!(control.validate_against(&agent()).is_err());
    assert_eq!(
        agent_receipt().outcome(),
        CommandOutcome::Failed {
            code: FailureCode::Unsupported
        }
    );
}
