use super::constants::*;
use super::hub::{LOCAL_HUB, PEER_HUB};
use crate::arguments::Arguments;
use crate::command::Command;
use clap::Parser;
use std::num::NonZeroU16;
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_enrollment_protocol::{InviteClaims, InviteDial, InvitePolicy};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationSnapshot, ConfigurationState, FIRST_CONFIGURATION_REVISION,
};
use yolo_tui_hub_cli::{HubCommand, PeerCommand, join_change};

fn local_identity() -> PeerIdentity {
    PeerIdentity::new(LOCAL_HUB.parse().unwrap(), [1; 32]).unwrap()
}

fn peer_identity() -> PeerIdentity {
    PeerIdentity::new(PEER_HUB.parse().unwrap(), [0x11; 32]).unwrap()
}

fn snapshot() -> ConfigurationSnapshot {
    ConfigurationSnapshot {
        state: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity: local_identity(),
            revision: FIRST_CONFIGURATION_REVISION,
            last_change: Uuid::new_v4(),
            configuration: Configuration::default(),
        },
        applied: true,
        listen_address: None,
    }
}

fn claims(policy: InvitePolicy) -> InviteClaims {
    InviteClaims {
        invite_id: Uuid::new_v4(),
        issuer: peer_identity(),
        dial: InviteDial {
            host: "10.0.0.2".into(),
            port: NonZeroU16::new(7443).unwrap(),
        },
        not_before_ms: 0,
        expires_at_ms: 600_000,
        policy,
    }
}

fn exported() -> InvitePolicy {
    InvitePolicy {
        export_local: true,
        import_peer: false,
    }
}

#[test]
fn join_parses_the_pairing_code_and_requires_one() {
    let arguments = Arguments::try_parse_from([
        PROGRAM,
        HUB_COMMAND,
        PEER_COMMAND,
        JOIN_COMMAND,
        "yolo1_code",
    ])
    .unwrap();
    assert!(matches!(
        arguments.command,
        Some(Command::Hub {
            command: HubCommand::Peer {
                command: PeerCommand::Join(_),
            },
        })
    ));
    assert!(
        Arguments::try_parse_from([PROGRAM, HUB_COMMAND, PEER_COMMAND, JOIN_COMMAND]).is_err()
    );
}

#[test]
fn join_pins_the_inviter_and_mirrors_the_policy() {
    let change = join_change(&snapshot(), &claims(exported())).unwrap().unwrap();
    let peer = &change.configuration.peers[0];
    assert_eq!(peer.identity, peer_identity());
    assert_eq!(peer.dial.as_ref().unwrap().host, "10.0.0.2");
    assert_eq!(peer.dial.as_ref().unwrap().port.get(), 7443);
    assert!(peer.consume);
    assert!(peer.exports.is_empty());

    let mutual = InvitePolicy {
        export_local: true,
        import_peer: true,
    };
    let change = join_change(&snapshot(), &claims(mutual)).unwrap().unwrap();
    let peer = &change.configuration.peers[0];
    let export = &peer.exports[0];
    assert_eq!(export.origin, LOCAL_HUB.parse().unwrap());
    assert_eq!(export.remaining_reexports, 0);
}

#[test]
fn join_is_idempotent_and_rejects_conflicts() {
    let claims = claims(exported());
    let change = join_change(&snapshot(), &claims).unwrap().unwrap();
    let mut applied = snapshot();
    applied.state.configuration = change.configuration.clone();
    assert!(join_change(&applied, &claims).unwrap().is_none());

    applied.state.configuration.peers[0].consume = false;
    assert!(join_change(&applied, &claims).is_err());
}

#[test]
fn join_rejects_a_local_invite() {
    let mut claims = claims(exported());
    claims.issuer = local_identity();
    assert!(join_change(&snapshot(), &claims).is_err());
}
