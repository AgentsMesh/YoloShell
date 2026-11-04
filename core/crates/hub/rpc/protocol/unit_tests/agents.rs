use crate::{commands::request, fixtures::*};
use uuid::Uuid;
use yolo_hub_agent_protocol::{AgentSnapshot, ObservationCursor, FIRST_OBSERVATION_SEQUENCE};
use yolo_hub_identity::{AgentId, ExecutionId};
use yolo_hub_rpc_protocol::{Command, Reply, Response};

#[test]
fn agent_queries_bind_exact_origin_and_address_not_only_the_shell() {
    let snapshot = agent();
    let mut address = snapshot.cursor().target().address;
    let request = request(Command::GetAgent {
        target: snapshot.cursor().target(),
    });
    Response::new(
        &request,
        hub(constants::HUB_ID),
        Reply::Agent {
            agent: snapshot.clone(),
        },
    )
    .validate_for(&request)
    .unwrap();
    address.agent_id = AgentId::new();
    let different =
        yolo_hub_rpc_protocol::Request::new(
            address.hub_id,
            Command::GetAgent {
                target: yolo_hub_identity::AgentTarget {
                    address,
                    execution_id: snapshot.cursor().target().execution_id,
                },
            },
        )
        .unwrap();
    assert!(
        Response::new(&different, address.hub_id, Reply::Agent { agent: snapshot })
            .validate_for(&different)
            .is_err()
    );
}

#[test]
fn aggregated_agent_pages_reject_duplicates_wrong_cursor_and_missing_continuation() {
    let snapshot = agent();
    let address = snapshot.cursor().target().address;
    let target = snapshot.cursor().target();
    let request = request(Command::ListAgents { after: None });
    let reply = Reply::Agents {
        agents: vec![snapshot.clone(), snapshot.clone()],
        next: None,
    };
    assert!(
        Response::new(&request, address.hub_id, reply)
            .validate_for(&request)
            .is_err()
    );
    let reply = Reply::Agents {
        agents: Vec::new(),
        next: Some(target),
    };
    assert!(
        Response::new(&request, address.hub_id, reply)
            .validate_for(&request)
            .is_err()
    );
    let request = yolo_hub_rpc_protocol::Request::new(
        address.hub_id,
        Command::ListAgents {
            after: Some(target),
        },
    )
    .unwrap();
    assert!(
        Response::new(
            &request,
            address.hub_id,
            Reply::Agents {
                agents: vec![snapshot],
                next: None
            }
        )
        .validate_for(&request)
        .is_err()
    );
}

#[test]
fn aggregated_agent_pages_keep_distinct_executions_with_one_logical_address() {
    let first = agent();
    let mut second_fields = first.fields().clone();
    let first_target = first.cursor().target();
    let second_target = yolo_hub_identity::AgentTarget {
        address: first_target.address,
        execution_id: ExecutionId::new(),
    };
    second_fields.cursor = ObservationCursor::new(
        second_target,
        Uuid::new_v4(),
        FIRST_OBSERVATION_SEQUENCE,
    )
    .unwrap();
    let second = AgentSnapshot::new(second_fields).unwrap();
    let request = request(yolo_hub_rpc_protocol::Command::ListAgents { after: None });
    let mut agents = vec![first.clone(), second.clone()];
    agents.sort_by_key(|snapshot| snapshot.cursor().target());
    let next = agents.last().unwrap().cursor().target();
    let response = yolo_hub_rpc_protocol::Response::new(
        &request,
        first_target.address.hub_id,
        yolo_hub_rpc_protocol::Reply::Agents {
            agents,
            next: Some(next),
        },
    );
    response.validate_for(&request).unwrap();
    assert_ne!(first_target.execution_id, second_target.execution_id);
}
