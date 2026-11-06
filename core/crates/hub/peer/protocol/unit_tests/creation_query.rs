use crate::{creation_query_support::*, fixtures::*, support::*};
use yolo_hub_delegation::FIRST_POLICY_EPOCH;
use yolo_hub_peer_protocol::{ExecutionReply, Purpose, Signed, required_rights};
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, Reply, Response};
use yolo_hub_terminal_protocol::{
    CreateShellRequest, CreationQueryOutcome, CreationQueryRequest, CreationQueryResult,
};

#[test]
fn creation_query_requires_lifecycle_at_every_hop_despite_being_read_only() {
    let fixture = Fixture::new();
    let query = query();
    for (rights, allowed) in [
        ([Right::Observe].into_iter().collect(), false),
        (
            [Right::Observe, Right::TerminalRead, Right::TerminalWrite]
                .into_iter()
                .collect(),
            false,
        ),
        ([Right::Lifecycle].into_iter().collect(), true),
    ] {
        let call = delegated(&fixture, query.clone(), rights);
        assert!(!call.payload.request.fields().command.mutates());
        assert_eq!(
            required_rights(&call.payload.request.fields().command).expect("remote query rights"),
            [Right::Lifecycle].into_iter().collect()
        );
        for (local, peer) in [
            (fixture.relay.identity, fixture.entry.identity),
            (fixture.origin.identity, fixture.relay.identity),
        ] {
            assert_eq!(
                call.admit(
                    local.hub(),
                    peer,
                    fixture.origin.identity,
                    FIRST_POLICY_EPOCH,
                    NOW
                )
                .is_ok(),
                allowed
            );
        }
    }
}

#[test]
fn query_signature_and_signed_reply_cannot_be_reused_for_another_creation_identity() {
    let fixture = Fixture::new();
    let original = query();
    let call = delegated(
        &fixture,
        original.clone(),
        [Right::Lifecycle].into_iter().collect(),
    );
    let mut altered = call.clone();
    let mut fields = altered.payload.request.fields().clone();
    let mut creation = original.creation().fields().clone();
    creation.creation_token = uuid::Uuid::new_v4();
    fields.command = Command::QueryCreation {
        request: CreationQueryRequest::new(CreateShellRequest::new(creation).unwrap()),
    };
    altered.payload.request = fields.try_into().unwrap();
    assert!(
        altered
            .admit(
                fixture.origin.identity.hub(),
                fixture.relay.identity,
                fixture.origin.identity,
                FIRST_POLICY_EPOCH,
                NOW
            )
            .is_err()
    );
    let reply = Reply::CreationQuery {
        query: CreationQueryResult::new(
            original.creation().fields().creation_token,
            CreationQueryOutcome::Missing {},
        )
        .unwrap(),
    };
    let signed = Signed::new(
        ExecutionReply {
            call_fingerprint: call.fingerprint().unwrap(),
            response: Response::new(&call.payload.request, fixture.origin.identity.hub(), reply),
        },
        Purpose::Response,
        &fixture.origin,
    )
    .unwrap();
    signed.validate_for(&call).unwrap();
    let other = delegated(&fixture, query(), [Right::Lifecycle].into_iter().collect());
    assert!(signed.validate_for(&other).is_err());
}
