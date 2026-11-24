use crate::{
    fixtures::{Fixture, agent_control},
    support::*,
};
use agent_control::constants::*;
use yolo_hub_delegation::FIRST_POLICY_EPOCH;
use yolo_hub_peer_protocol::{Call, PEER_PROTOCOL_VERSION, PeerRequest, Signed, required_rights};
use yolo_hub_routing::{Right, Rights};
use yolo_hub_rpc_protocol::{Command, RPC_METHOD_TAG, RPC_VERSION_FIELD};

#[test]
fn semantic_control_requires_two_independent_rights_on_every_delegation_hop() {
    let fixture = Fixture::new();
    let required: Rights = [Right::AgentControl, Right::TerminalWrite]
        .into_iter()
        .collect();
    let command = agent_control::command(&fixture);
    assert_eq!(
        required_rights(&command).expect("remote control rights"),
        required
    );
    for granted in [
        [Right::Observe].into_iter().collect(),
        [Right::AgentControl].into_iter().collect(),
        [Right::TerminalWrite].into_iter().collect(),
        [Right::Lifecycle].into_iter().collect(),
        [Right::FileRead].into_iter().collect(),
        [Right::Observe, Right::TerminalRead].into_iter().collect(),
    ] {
        let call = agent_control::call(&fixture, required.union(&granted), granted);
        assert!(
            call.admit(
                fixture.relay.identity.hub(),
                fixture.entry.identity,
                fixture.origin.identity,
                FIRST_POLICY_EPOCH,
                NOW
            )
            .is_err()
        );
        assert!(
            call.admit(
                fixture.origin.identity.hub(),
                fixture.relay.identity,
                fixture.origin.identity,
                FIRST_POLICY_EPOCH,
                NOW
            )
            .is_err()
        );
    }
    let call = agent_control::call(&fixture, required.clone(), required);
    call.admit(
        fixture.relay.identity.hub(),
        fixture.entry.identity,
        fixture.origin.identity,
        FIRST_POLICY_EPOCH,
        NOW,
    )
    .unwrap();
    call.admit(
        fixture.origin.identity.hub(),
        fixture.relay.identity,
        fixture.origin.identity,
        FIRST_POLICY_EPOCH,
        NOW,
    )
    .unwrap();
    assert_eq!(
        call.next_hop(fixture.entry.identity.hub()),
        Some(fixture.relay.identity.hub())
    );
}

#[test]
fn control_calls_keep_delivery_identity_through_serialization_and_reject_old_rpc_envelopes() {
    let fixture = Fixture::new();
    let required: Rights = [Right::AgentControl, Right::TerminalWrite]
        .into_iter()
        .collect();
    let call = agent_control::call(&fixture, required.clone(), required);
    let encoded = yolo_hub_transport::encode(&call).unwrap();
    let restored: Signed<Call> =
        yolo_hub_transport::decode(&encoded[yolo_hub_transport::FRAME_LENGTH_BYTES..]).unwrap();
    assert_eq!(restored, call);
    let Command::AgentControl { request, .. } = &restored.payload.request.fields().command else {
        panic!()
    };
    assert_eq!(
        restored.payload.request.fields().request_id,
        request.fields().request_id
    );
    let mut old = serde_json::to_value(&call).unwrap();
    assert_eq!(
        old[PAYLOAD_FIELD][REQUEST_FIELD][COMMAND_FIELD][RPC_METHOD_TAG],
        CONTROL_METHOD
    );
    old[PAYLOAD_FIELD][REQUEST_FIELD][RPC_VERSION_FIELD] = PREVIOUS_RPC_VERSION.into();
    assert!(serde_json::from_value::<Signed<Call>>(old).is_err());
    assert!(PEER_PROTOCOL_VERSION > PREVIOUS_PEER_VERSION);
    let discovery = PeerRequest::Discover {
        request_id: uuid::Uuid::new_v4(),
        version: PEER_PROTOCOL_VERSION,
        allowed_origins: vec![fixture.origin.identity.hub()],
    };
    let decoded: PeerRequest =
        serde_json::from_slice(&serde_json::to_vec(&discovery).unwrap()).unwrap();
    assert_eq!(decoded, discovery);
}

#[test]
fn changing_a_grant_or_signature_invalidates_the_call() {
    let fixture = Fixture::new();
    let required: Rights = [Right::AgentControl, Right::TerminalWrite]
        .into_iter()
        .collect();
    let original = agent_control::call(&fixture, required.clone(), required);
    let mut changed = original.clone();
    let mut fields = original.payload.request.fields().clone();
    if let Command::AgentControl { grant, .. } = &mut fields.command {
        let mut replacement = *grant.fields();
        replacement.capability = yolo_hub_terminal_protocol::SecretCapability::generate();
        *grant = yolo_hub_terminal_protocol::LeaseGrant::new(replacement).unwrap();
    }
    changed.payload.request = yolo_hub_rpc_protocol::Request::try_from(fields).unwrap();
    assert_ne!(
        changed.fingerprint().unwrap(),
        original.fingerprint().unwrap()
    );
    assert!(
        changed
            .admit(
                fixture.origin.identity.hub(),
                fixture.relay.identity,
                fixture.origin.identity,
                FIRST_POLICY_EPOCH,
                NOW
            )
            .is_err()
    );
    let mut wire = serde_json::to_value(&original).unwrap();
    wire[SIGNATURE_FIELD] = serde_json::json!([]);
    let unsigned: Signed<Call> = serde_json::from_value(wire).unwrap();
    assert!(
        unsigned
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
