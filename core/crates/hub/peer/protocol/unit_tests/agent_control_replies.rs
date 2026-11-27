use crate::{
    fixtures::{Fixture, agent_control},
    support::*,
};
use yolo_hub_agent_protocol::{CommandOutcome, CommandReceipt, FailureCode};
use yolo_hub_identity::{AgentId, ExecutionId};
use yolo_hub_peer_protocol::{ExecutionReply, Purpose, Signed};
use yolo_hub_routing::{Right, Rights};
use yolo_hub_rpc_protocol::{Command, Reply, Response};

#[test]
fn even_an_origin_signed_reply_must_match_exact_agent_execution_and_request_id() {
    let fixture = Fixture::new();
    let required: Rights = [Right::AgentControl, Right::TerminalWrite]
        .into_iter()
        .collect();
    let call = agent_control::call(&fixture, required.clone(), required);
    let Command::AgentControl { request, .. } = &call.payload.request.fields().command else {
        panic!()
    };
    let exact = request.fields().target;
    let mut other_agent = exact;
    other_agent.address.agent_id = AgentId::new();
    let mut other_execution = exact;
    other_execution.execution_id = ExecutionId::new();
    let mut other_origin = exact;
    other_origin.address.hub_id = fixture.entry.identity.hub();
    for (target, request_id) in [
        (exact, request.fields().request_id),
        (other_agent, request.fields().request_id),
        (other_execution, request.fields().request_id),
        (other_origin, request.fields().request_id),
        (exact, uuid::Uuid::new_v4()),
    ] {
        let mut receipt = CommandReceipt::accepted(request_id, target).unwrap();
        receipt
            .advance(CommandOutcome::Failed {
                code: FailureCode::Unsupported,
            })
            .unwrap();
        let response = Signed::new(
            ExecutionReply {
                call_fingerprint: call.fingerprint().unwrap(),
                response: Response::new(
                    &call.payload.request,
                    fixture.origin.identity.hub(),
                    Reply::AgentControl { receipt },
                ),
            },
            Purpose::Response,
            &fixture.origin,
        )
        .unwrap();
        assert_eq!(
            response.validate_for(&call).is_ok(),
            target == exact && request_id == request.fields().request_id
        );
    }
}

#[test]
fn a_relay_cannot_sign_an_origin_semantic_control_result() {
    let fixture = Fixture::new();
    let required: Rights = [Right::AgentControl, Right::TerminalWrite]
        .into_iter()
        .collect();
    let call = agent_control::call(&fixture, required.clone(), required);
    let Command::AgentControl { request, .. } = &call.payload.request.fields().command else {
        panic!()
    };
    let receipt =
        CommandReceipt::accepted(request.fields().request_id, request.fields().target).unwrap();
    let response = Signed::new(
        ExecutionReply {
            call_fingerprint: call.fingerprint().unwrap(),
            response: Response::new(
                &call.payload.request,
                fixture.origin.identity.hub(),
                Reply::AgentControl { receipt },
            ),
        },
        Purpose::Response,
        &fixture.relay,
    )
    .unwrap();
    assert!(response.validate_for(&call).is_err());
    assert_eq!(call.payload.issued_at_ms, NOW);
}
