use crate::{fixtures::*, support::*};
use yolo_hub_delegation::{DelegationChain, FIRST_POLICY_EPOCH, SignedGrant};
use yolo_hub_peer_protocol::{Call, Purpose, Signed, required_rights};
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::{CreateShellResult, SecretCapability};

#[test]
fn cleanup_creation_requires_lifecycle_delegation_in_addition_to_its_capability() {
    let fixture = Fixture::new();
    let creation = CreateShellResult {
        address: fixture
            .export
            .catalogue
            .payload
            .terminals
            .first()
            .unwrap()
            .fields()
            .address,
        creation_capability: SecretCapability::generate(),
    };
    let lifecycle: yolo_hub_routing::Rights = [Right::Lifecycle].into_iter().collect();
    for command in [
        Command::CleanupTerminate {
            creation,
            force: true,
        },
        Command::CleanupArchive { creation },
    ] {
        assert_eq!(
            required_rights(&command).expect("remote cleanup rights"),
            lifecycle
        );
        let request = Request::new(fixture.origin.identity.hub(), command).unwrap();
        let insufficient = Signed::new(
            Call {
                request: request.clone(),
                authority: fixture.export.authority.clone(),
                issued_at_ms: NOW,
                expires_at_ms: EXPIRES,
            },
            Purpose::Call,
            &fixture.entry,
        )
        .unwrap();
        assert!(
            insufficient
                .admit(
                    fixture.origin.identity.hub(),
                    fixture.relay.identity,
                    fixture.origin.identity,
                    FIRST_POLICY_EPOCH,
                    NOW
                )
                .is_err()
        );
        let mut claims = root_claims(&fixture.origin, &fixture.relay);
        claims.rights = lifecycle.clone();
        let root = SignedGrant::issue(claims, &fixture.origin).unwrap();
        let child =
            SignedGrant::issue(child_claims(&root, &fixture.entry), &fixture.relay).unwrap();
        let authorized = Signed::new(
            Call {
                request,
                authority: DelegationChain::new(vec![root, child]).unwrap(),
                issued_at_ms: NOW,
                expires_at_ms: EXPIRES,
            },
            Purpose::Call,
            &fixture.entry,
        )
        .unwrap();
        authorized
            .admit(
                fixture.origin.identity.hub(),
                fixture.relay.identity,
                fixture.origin.identity,
                FIRST_POLICY_EPOCH,
                NOW,
            )
            .unwrap();
        for change_capability in [false, true] {
            let mut tampered = authorized.clone();
            let mut fields = tampered.payload.request.fields().clone();
            let selected = match &mut fields.command {
                Command::CleanupTerminate { creation, .. }
                | Command::CleanupArchive { creation } => creation,
                _ => unreachable!(),
            };
            if change_capability {
                selected.creation_capability = SecretCapability::generate();
            } else {
                selected.address.terminal_id = yolo_hub_identity::TerminalId::new();
            }
            tampered.payload.request = fields.try_into().unwrap();
            assert!(
                tampered
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
    }
}
