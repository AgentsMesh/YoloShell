use crate::{
    fixtures::{Fixture, agent_control},
    support::*,
};
use agent_control::constants::INTERACTION_CHOICE;
use yolo_hub_agent_protocol::{
    AgentAction, AgentControlRequest, InteractionReference, InteractionResponse,
};
use yolo_hub_delegation::{DelegationChain, FIRST_POLICY_EPOCH, SignedGrant};
use yolo_hub_routing::{Right, Rights};
use yolo_hub_rpc_protocol::{Command, Request};

#[test]
fn signed_interrupt_cannot_be_changed_into_an_interaction_response() {
    let fixture = Fixture::new();
    let required: Rights = [Right::AgentControl, Right::TerminalWrite]
        .into_iter()
        .collect();
    let original = agent_control::call(&fixture, required.clone(), required);
    let mut fields = original.payload.request.fields().clone();
    let Command::AgentControl { request, .. } = &mut fields.command else {
        panic!()
    };
    let mut control = request.fields().clone();
    control.action = AgentAction::Respond {
        interaction: InteractionReference {
            target: control.target,
            id: uuid::Uuid::new_v4(),
        },
        response: InteractionResponse::Choice {
            choice: INTERACTION_CHOICE.into(),
        },
    };
    *request = AgentControlRequest::new(control).unwrap();
    let mut forged = original.clone();
    forged.payload.request = Request::try_from(fields).unwrap();
    assert_ne!(
        original.fingerprint().unwrap(),
        forged.fingerprint().unwrap()
    );
    assert!(
        forged
            .admit(
                fixture.origin.identity.hub(),
                fixture.relay.identity,
                fixture.origin.identity,
                FIRST_POLICY_EPOCH,
                NOW
            )
            .is_err()
    );
}

#[test]
fn relay_cannot_add_the_independent_semantic_right_to_an_older_terminal_only_grant() {
    let fixture = Fixture::new();
    let mut claims = root_claims(&fixture.origin, &fixture.relay);
    claims.rights = [Right::TerminalWrite].into_iter().collect();
    let root = SignedGrant::issue(claims, &fixture.origin).unwrap();
    let mut claims = child_claims(&root, &fixture.entry);
    claims.rights = [Right::TerminalWrite, Right::AgentControl]
        .into_iter()
        .collect();
    let escalated = SignedGrant::issue(claims, &fixture.relay).unwrap();
    assert!(DelegationChain::new(vec![root, escalated]).is_err());
}
