use super::{Fixture, support::*};
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    FIRST_OBSERVATION_SEQUENCE, ObservationCursor,
};
use yolo_hub_delegation::{DelegationChain, SignedGrant};
use yolo_hub_identity::{AgentAddress, AgentId, AgentTarget, ExecutionId};
use yolo_hub_peer_protocol::{Call, Purpose, Signed};
use yolo_hub_routing::Rights;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::{
    FIRST_LEASE_GENERATION, LeaseGrant, LeaseGrantFields, SecretCapability, WriterPrincipal,
};

#[path = "agent_control_constants.rs"]
pub mod constants;
use constants::*;

pub fn command(fixture: &Fixture) -> Command {
    let terminal = fixture.export.catalogue.payload.terminals[0]
        .fields()
        .runtime
        .unwrap()
        .target;
    let target = AgentTarget {
        address: AgentAddress {
            hub_id: fixture.origin.identity.hub(),
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    };
    let request = AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        target,
        expected_cursor: ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE)
            .unwrap(),
        action: AgentAction::Interrupt {},
    })
    .unwrap();
    let grant = LeaseGrant::new(LeaseGrantFields {
        target: terminal,
        principal: WriterPrincipal {
            client_id: Uuid::new_v4(),
            controller_instance_id: Uuid::new_v4(),
        },
        generation: FIRST_LEASE_GENERATION,
        capability: SecretCapability::generate(),
        expires_at_ms: EXPIRES as i64,
        ttl_ms: LEASE_TTL_MS,
        renew_after_ms: LEASE_RENEW_MS,
    })
    .unwrap();
    Command::AgentControl { request, grant }
}

pub fn call(fixture: &Fixture, root_rights: Rights, recipient_rights: Rights) -> Signed<Call> {
    let mut claims = root_claims(&fixture.origin, &fixture.relay);
    claims.rights = root_rights;
    let root = SignedGrant::issue(claims, &fixture.origin).unwrap();
    let mut claims = child_claims(&root, &fixture.entry);
    claims.rights = recipient_rights;
    let child = SignedGrant::issue(claims, &fixture.relay).unwrap();
    Signed::new(
        Call {
            request: Request::new(fixture.origin.identity.hub(), command(fixture)).unwrap(),
            authority: DelegationChain::new(vec![root, child]).unwrap(),
            issued_at_ms: NOW,
            expires_at_ms: EXPIRES,
        },
        Purpose::Call,
        &fixture.entry,
    )
    .unwrap()
}
