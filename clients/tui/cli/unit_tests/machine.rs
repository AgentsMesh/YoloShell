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
use yolo_tui_hub_cli::{MachineCommand, PinMachine, pin_change};

pub const MACHINE_HUB: &str = "11111111-2222-4333-8444-555555555555";
pub const LOCAL_HUB: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
pub const MACHINE_KEY: &str = "1111111111111111111111111111111111111111111111111111111111111111";
pub const INVALID_KEY: &str = "abc";
pub const PROFILE_REVISION: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn local_identity() -> PeerIdentity {
    PeerIdentity::new(LOCAL_HUB.parse().unwrap(), [1; 32]).unwrap()
}

fn machine_identity() -> PeerIdentity {
    PeerIdentity::new(MACHINE_HUB.parse().unwrap(), [0x11; 32]).unwrap()
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

fn parse(arguments: Vec<&str>) -> PinMachine {
    let arguments = Arguments::try_parse_from(arguments).unwrap();
    match arguments.command {
        Some(Command::Machine {
            command: MachineCommand::Pin(args),
        }) => args,
        _ => panic!("expected machine pin"),
    }
}

fn pin_args(extra: &[&str]) -> PinMachine {
    let mut arguments = vec![
        PROGRAM,
        MACHINE_COMMAND,
        PIN_COMMAND,
        MACHINE_HUB,
        MACHINE_KEY,
    ];
    arguments.extend_from_slice(extra);
    parse(arguments)
}

#[test]
fn pin_accepts_an_ssh_target_and_remote_socket_together() {
    let args = pin_args(&[
        SSH_ARGUMENT,
        SSH_TARGET,
        REMOTE_SOCKET_ARGUMENT,
        REMOTE_SOCKET,
    ]);
    let change = pin_change(&snapshot(), &args).unwrap();
    let dial = change.configuration.peers[0].dial.as_ref().unwrap();
    assert_eq!(dial.ssh_target(), Some(SSH_TARGET));
    assert_eq!(dial.socket().to_str(), Some(REMOTE_SOCKET));
    assert!(
        Arguments::try_parse_from([
            PROGRAM,
            MACHINE_COMMAND,
            PIN_COMMAND,
            MACHINE_HUB,
            MACHINE_KEY,
            SSH_ARGUMENT,
            SSH_TARGET,
        ])
        .is_err()
    );
    assert!(
        Arguments::try_parse_from([
            PROGRAM,
            MACHINE_COMMAND,
            PIN_COMMAND,
            MACHINE_HUB,
            MACHINE_KEY,
            REMOTE_SOCKET_ARGUMENT,
            REMOTE_SOCKET,
        ])
        .is_err()
    );
}

#[test]
fn pins_a_machine_with_an_ssh_dial_and_no_exports_by_default() {
    let change = pin_change(
        &snapshot(),

        &pin_args(&[
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
    assert_eq!(peer.identity, machine_identity());
    assert!(peer.consume);
    let dial = peer.dial.as_ref().unwrap();
    assert_eq!(dial.ssh_target(), Some(SSH_TARGET));
    assert_eq!(dial.socket().to_str(), Some(REMOTE_SOCKET));
    assert!(peer.exports.is_empty());
}

#[test]
fn pin_only_and_local_export_are_applied() {
    let change = pin_change(
        &snapshot(),

        &pin_args(&[
            PIN_ONLY_ARGUMENT,
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
    let args = pin_args(&[]);
    let mut existing = snapshot();
    existing
        .state
        .configuration
        .peers
        .push(yolo_hub_federation_settings::Peer {
            identity: machine_identity(),
            dial: None,
            consume: false,
            exports: vec![],
        });
    assert!(pin_change(&existing, &args).is_err());

    let mut self_snapshot = snapshot();
    self_snapshot.state.identity = local_identity_at_machine_hub();
    assert!(pin_change(&self_snapshot, &args).is_err());

    let invalid = parse(vec![
        PROGRAM,
        MACHINE_COMMAND,
        PIN_COMMAND,
        MACHINE_HUB,
        INVALID_KEY,
    ]);
    assert!(pin_change(&snapshot(), &invalid).is_err());

}

fn local_identity_at_machine_hub() -> PeerIdentity {
    PeerIdentity::new(MACHINE_HUB.parse().unwrap(), [3; 32]).unwrap()
}

