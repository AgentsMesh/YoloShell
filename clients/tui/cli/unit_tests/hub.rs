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
use yolo_tui_hub_cli::{AddPeer, HubCommand, PeerCommand, add_change};

pub const PEER_HUB: &str = "11111111-2222-4333-8444-555555555555";
pub const LOCAL_HUB: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
pub const PEER_KEY: &str = "1111111111111111111111111111111111111111111111111111111111111111";
pub const INVALID_KEY: &str = "abc";
pub const PROFILE_REVISION: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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
        peer_socket: Some("/run/user/1000/yolo/hub/peer.sock".into()),
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

#[test]
fn add_accepts_an_ssh_target_and_remote_socket_together() {
    let args = add_args(&[
        SSH_ARGUMENT,
        SSH_TARGET,
        REMOTE_SOCKET_ARGUMENT,
        REMOTE_SOCKET,
    ]);
    let change = add_change(&snapshot(), &args).unwrap();
    let dial = change.configuration.peers[0].dial.as_ref().unwrap();
    assert_eq!(dial.ssh_target(), Some(SSH_TARGET));
    assert_eq!(dial.socket().to_str(), Some(REMOTE_SOCKET));
    assert!(
        Arguments::try_parse_from([
            PROGRAM,
            HUB_COMMAND,
            PEER_COMMAND,
            ADD_COMMAND,
            PEER_HUB,
            PEER_KEY,
            SSH_ARGUMENT,
            SSH_TARGET,
        ])
        .is_err()
    );
    assert!(
        Arguments::try_parse_from([
            PROGRAM,
            HUB_COMMAND,
            PEER_COMMAND,
            ADD_COMMAND,
            PEER_HUB,
            PEER_KEY,
            REMOTE_SOCKET_ARGUMENT,
            REMOTE_SOCKET,
        ])
        .is_err()
    );
}

#[test]
fn adds_a_pinned_peer_with_an_ssh_dial_and_no_exports_by_default() {
    let change = add_change(
        &snapshot(),

        &add_args(&[
            SSH_ARGUMENT,
            SSH_TARGET,
            REMOTE_SOCKET_ARGUMENT,
            REMOTE_SOCKET,
        ]),
    )
    .unwrap();
    assert_eq!(change.expected_revision, FIRST_CONFIGURATION_REVISION);
    assert!(!change.request_id.is_nil());
    let peer = &change.configuration.peers[0];
    assert_eq!(peer.identity, peer_identity());
    assert!(peer.consume);
    let dial = peer.dial.as_ref().unwrap();
    assert_eq!(dial.ssh_target(), Some(SSH_TARGET));
    assert_eq!(dial.socket().to_str(), Some(REMOTE_SOCKET));
    assert!(peer.exports.is_empty());
}

#[test]
fn no_import_and_local_export_are_applied() {
    let change = add_change(
        &snapshot(),

        &add_args(&[
            NO_IMPORT_ARGUMENT,
            SSH_ARGUMENT,
            SSH_TARGET,
            REMOTE_SOCKET_ARGUMENT,
            REMOTE_SOCKET,
            EXPORT_LOCAL_ARGUMENT,
        ]),
    )
    .unwrap();
    let peer = &change.configuration.peers[0];
    assert!(!peer.consume);
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
    let args = add_args(&[]);
    let mut existing = snapshot();
    existing
        .state
        .configuration
        .peers
        .push(yolo_hub_federation_settings::Peer {
            identity: peer_identity(),
            dial: None,
            consume: false,
            exports: vec![],
        });
    assert!(add_change(&existing, &args).is_err());

    let mut self_snapshot = snapshot();
    self_snapshot.state.identity = local_identity_at_peer_hub();
    assert!(add_change(&self_snapshot, &args).is_err());

    let invalid = parse(vec![
        PROGRAM,
        HUB_COMMAND,
        PEER_COMMAND,
        ADD_COMMAND,
        PEER_HUB,
        INVALID_KEY,
    ]);
    assert!(add_change(&snapshot(), &invalid).is_err());

}

fn local_identity_at_peer_hub() -> PeerIdentity {
    PeerIdentity::new(PEER_HUB.parse().unwrap(), [3; 32]).unwrap()
}

