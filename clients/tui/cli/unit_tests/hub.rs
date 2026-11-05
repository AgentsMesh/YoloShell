use super::constants::*;
use crate::arguments::Arguments;
use crate::command::Command;
use clap::Parser;
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationSnapshot, ConfigurationState, FIRST_CONFIGURATION_REVISION,
};
use yolo_hub_routing::Right;
use yolo_tui_hub_cli::{
    AddPeer, HubCommand, InvitePeer, PeerCommand, add_change, invite_request,
};

pub const PEER_HUB: &str = "11111111-2222-4333-8444-555555555555";
pub const LOCAL_HUB: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
pub const PEER_KEY: &str = "1111111111111111111111111111111111111111111111111111111111111111";
pub const INVALID_KEY: &str = "abc";

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

fn parse(arguments: Vec<&str>) -> AddPeer {
    let arguments = Arguments::try_parse_from(arguments).unwrap();
    match arguments.command {
        Some(Command::Hub {
            command:
                HubCommand::Peer {
                    command: PeerCommand::Add(args),
                },
        }) => args,
        _ => panic!("expected hub peer add"),
    }
}

fn add_args(extra: &[&str]) -> AddPeer {
    let mut arguments = vec![
        PROGRAM,
        HUB_COMMAND,
        PEER_COMMAND,
        ADD_COMMAND,
        PEER_HUB,
        PEER_KEY,
    ];
    arguments.extend_from_slice(extra);
    parse(arguments)
}

fn add_args_with_key(key: &str) -> AddPeer {
    parse(vec![
        PROGRAM,
        HUB_COMMAND,
        PEER_COMMAND,
        ADD_COMMAND,
        PEER_HUB,
        key,
    ])
}

fn invite_args(extra: &[&str]) -> InvitePeer {
    let mut arguments = vec![PROGRAM, HUB_COMMAND, PEER_COMMAND, INVITE_COMMAND];
    arguments.extend_from_slice(extra);
    let arguments = Arguments::try_parse_from(arguments).unwrap();
    match arguments.command {
        Some(Command::Hub {
            command:
                HubCommand::Peer {
                    command: PeerCommand::Invite(args),
                },
        }) => args,
        _ => panic!("expected hub peer invite"),
    }
}

#[test]
fn invite_is_address_free_and_honors_policy_flags() {
    let request = invite_request(&invite_args(&[])).unwrap();
    assert_eq!(request.dial, None);
    assert_eq!(request.lifetime_ms, 10 * 60 * 1000);
    assert!(request.policy.export_local);
    assert!(!request.policy.import_peer);

    let request = invite_request(&invite_args(&[
        NO_EXPORT_ARGUMENT,
        IMPORT_PEER_ARGUMENT,
        TTL_ARGUMENT,
        "5",
    ]))
    .unwrap();
    assert_eq!(request.dial, None);
    assert_eq!(request.lifetime_ms, 5 * 60 * 1000);
    assert!(!request.policy.export_local);
    assert!(request.policy.import_peer);
}

#[test]
fn adds_a_peer_with_import_and_no_exports_by_default() {
    let change = add_change(&snapshot(), &add_args(&[])).unwrap();
    assert_eq!(change.expected_revision, FIRST_CONFIGURATION_REVISION);
    assert!(!change.request_id.is_nil());
    assert_eq!(change.configuration.peers.len(), 1);
    let peer = &change.configuration.peers[0];
    assert_eq!(peer.identity, peer_identity());
    assert!(peer.consume);
    assert!(peer.dial.is_none());
    assert!(peer.exports.is_empty());
}

#[test]
fn no_import_dial_and_local_export_are_applied() {
    let change = add_change(
        &snapshot(),
        &add_args(&[
            NO_IMPORT_ARGUMENT,
            HOST_ARGUMENT,
            "10.0.0.2",
            PORT_ARGUMENT,
            "7443",
            EXPORT_LOCAL_ARGUMENT,
        ]),
    )
    .unwrap();
    let peer = &change.configuration.peers[0];
    assert!(!peer.consume);
    let dial = peer.dial.as_ref().unwrap();
    assert_eq!(dial.host, "10.0.0.2");
    assert_eq!(dial.port.get(), 7443);
    assert_eq!(peer.exports.len(), 1);
    assert_eq!(peer.exports[0].origin, LOCAL_HUB.parse().unwrap());
    assert!(
        peer.exports[0]
            .rights
            .allows(&[Right::Observe].into_iter().collect())
    );
    assert_eq!(peer.exports[0].remaining_reexports, 0);
}

#[test]
fn rejects_duplicate_self_and_invalid_keys() {
    let mut existing = snapshot();
    existing
        .state
        .configuration
        .peers
        .push(yolo_hub_federation_settings::Peer {
            identity: peer_identity(),
            dial: None,
            consume: true,
            exports: vec![],
        });
    assert!(add_change(&existing, &add_args(&[])).is_err());

    let self_args = add_args(&[]);
    let mut self_snapshot = snapshot();
    self_snapshot.state.identity = local_identity_at_peer_hub();
    assert!(add_change(&self_snapshot, &self_args).is_err());

    assert!(add_change(&snapshot(), &add_args_with_key(INVALID_KEY)).is_err());
}

fn local_identity_at_peer_hub() -> PeerIdentity {
    PeerIdentity::new(PEER_HUB.parse().unwrap(), [3; 32]).unwrap()
}
