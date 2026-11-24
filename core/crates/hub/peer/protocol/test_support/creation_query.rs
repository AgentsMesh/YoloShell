use crate::{fixtures::*, support::*};
use yolo_hub_delegation::{DelegationChain, SignedGrant};
use yolo_hub_peer_protocol::{Call, Purpose, Signed};
use yolo_hub_routing::Rights;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::*;

pub fn query() -> CreationQueryRequest {
    CreationQueryRequest::new(
        CreateShellRequest::new(CreateShellFields {
            schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
            request_id: uuid::Uuid::new_v4(),
            creation_token: uuid::Uuid::new_v4(),
            name: Some(SESSION_NAME.into()),
            cwd: DIRECTORY.into(),
            size: TerminalSize::default(),
        })
        .unwrap(),
    )
}
pub fn delegated(fixture: &Fixture, request: CreationQueryRequest, rights: Rights) -> Signed<Call> {
    let mut claims = root_claims(&fixture.origin, &fixture.relay);
    claims.rights = rights;
    let root = SignedGrant::issue(claims, &fixture.origin).unwrap();
    let child = SignedGrant::issue(child_claims(&root, &fixture.entry), &fixture.relay).unwrap();
    Signed::new(
        Call {
            request: Request::new(
                fixture.origin.identity.hub(),
                Command::QueryCreation { request },
            )
            .unwrap(),
            authority: DelegationChain::new(vec![root, child]).unwrap(),
            issued_at_ms: NOW,
            expires_at_ms: EXPIRES,
        },
        Purpose::Call,
        &fixture.entry,
    )
    .unwrap()
}
