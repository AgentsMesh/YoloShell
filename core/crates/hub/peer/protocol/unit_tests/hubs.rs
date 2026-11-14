use crate::{fixtures::*, support::*};
use yolo_hub_delegation::FIRST_POLICY_EPOCH;
use yolo_hub_peer_protocol::{Call, Purpose, Signed, required_rights};
use yolo_hub_rpc_protocol::{Command, Request};

#[test]
fn hub_inventory_requires_observe_authority_through_each_hop() {
    let fixture = Fixture::new();
    let command = Command::ListHubs {
        after: Some(fixture.relay.identity.hub()),
    };
    assert_eq!(
        required_rights(&command).expect("remote observe rights"),
        observe()
    );
    let request = Request::new(fixture.origin.identity.hub(), command).unwrap();
    let call = Signed::new(
        Call {
            request,
            authority: fixture.export.authority,
            issued_at_ms: NOW,
            expires_at_ms: EXPIRES,
        },
        Purpose::Call,
        &fixture.entry,
    )
    .unwrap();
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
    assert!(
        call.admit(
            fixture.origin.identity.hub(),
            fixture.entry.identity,
            fixture.origin.identity,
            FIRST_POLICY_EPOCH,
            NOW
        )
        .is_err()
    );
}
