use super::{AddPeer, add_change};
use std::path::PathBuf;
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationSnapshot, ConfigurationState,
};

mod constants;
use constants::*;

fn snapshot() -> ConfigurationSnapshot {
    ConfigurationSnapshot {
        state: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity: PeerIdentity::new(HubId::new(), LOCAL_KEY).unwrap(),
            revision: 1,
            last_change: Uuid::new_v4(),
            configuration: Configuration::default(),
        },
        applied: true,
        peer_socket: Some(SOCKET.into()),
    }
}

fn args(ssh_profile: Option<&str>, remote_socket: Option<&str>) -> AddPeer {
    AddPeer {
        peer: HubId::new(),
        key: PEER_KEY.into(),
        ssh_profile: ssh_profile.map(str::to_owned),
        remote_socket: remote_socket.map(PathBuf::from),
        no_import: false,
        exports: vec![],
        export_local: true,
        json: false,
    }
}

#[test]
fn adds_a_pinned_peer_with_an_ssh_socket_dial_and_exports() {
    let change = add_change(&snapshot(), &args(Some(PROFILE), Some(SOCKET))).unwrap();
    let peer = &change.configuration.peers[0];
    let dial = peer.dial.as_ref().unwrap();
    assert_eq!(dial.profile, PROFILE);
    assert_eq!(dial.socket, PathBuf::from(SOCKET));
    assert!(peer.consume);
    assert_eq!(peer.exports.len(), 1);
    assert_eq!(change.expected_revision, 1);
}

#[test]
fn dial_flags_are_paired_and_the_key_is_validated() {
    assert!(add_change(&snapshot(), &args(None, Some(SOCKET))).is_err());
    assert!(add_change(&snapshot(), &args(Some(PROFILE), None)).is_err());
    let mut invalid = args(None, None);
    invalid.key = "not-a-key".into();
    assert!(add_change(&snapshot(), &invalid).is_err());
}

#[test]
fn a_peer_can_be_pinned_without_a_dial() {
    let change = add_change(&snapshot(), &args(None, None)).unwrap();
    assert!(change.configuration.peers[0].dial.is_none());
}
