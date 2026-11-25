use crate::{fixtures::*, support::*};
use yolo_hub_delegation::FIRST_POLICY_EPOCH;
use yolo_hub_peer_protocol::{Call, Purpose, Signed, required_rights};
use yolo_hub_rpc_protocol::{Command, Request};

#[test]
fn local_configuration_is_not_admitted_even_with_a_valid_signed_delegation() {
    let fixture = Fixture::new();
    let commands = [
        Command::GetBuild {},
        Command::GetConfiguration {},
        Command::InspectRemoteSetup {
            operation_id: uuid::Uuid::new_v4(),
        },
        Command::Configure {
            change: yolo_hub_management_protocol::ConfigurationChange {
                request_id: uuid::Uuid::new_v4(),
                expected_revision: yolo_hub_management_protocol::FIRST_CONFIGURATION_REVISION,
                configuration: yolo_hub_federation_settings::Configuration::default(),
            },
        },
    ];
    for command in commands {
        assert!(required_rights(&command).is_none());
        let request = Request::new(fixture.origin.identity.hub(), command).unwrap();
        let call = Signed::new(
            Call {
                request,
                authority: fixture.export.authority.clone(),
                issued_at_ms: NOW,
                expires_at_ms: EXPIRES,
            },
            Purpose::Call,
            &fixture.entry,
        )
        .unwrap();
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
}
