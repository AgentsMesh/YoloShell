use super::{GrantMachine, PinMachine, grant_change, import_change, pin_change, remove_change};
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

fn args(ssh: Option<&str>, remote_socket: Option<&str>) -> PinMachine {
    PinMachine {
        machine: HubId::new(),
        key: PEER_KEY.into(),
        ssh: ssh.map(str::to_owned),
        remote_socket: remote_socket.map(PathBuf::from),
        pin_only: false,
        exports: vec![],
        export_local: true,
        json: false,
    }
}

#[test]
fn pins_a_source_with_an_ssh_socket_dial_and_exports() {
    let change = pin_change(&snapshot(), &args(Some(PROFILE), Some(SOCKET))).unwrap();
    let peer = &change.configuration.peers[0];
    let dial = peer.dial.as_ref().unwrap();
    assert_eq!(dial.ssh_target(), Some(PROFILE));
    assert_eq!(dial.socket(), PathBuf::from(SOCKET));
    assert!(peer.consume);
    assert_eq!(peer.exports.len(), 1);
    assert_eq!(change.expected_revision, 1);
}

#[test]
fn dial_flags_are_paired_and_the_key_is_validated() {
    assert!(pin_change(&snapshot(), &args(None, Some(SOCKET))).is_err());
    assert!(pin_change(&snapshot(), &args(Some(PROFILE), None)).is_err());
    let mut invalid = args(None, None);
    invalid.key = "not-a-key".into();
    assert!(pin_change(&snapshot(), &invalid).is_err());
}

#[test]
fn a_source_can_be_pinned_without_a_dial() {
    let change = pin_change(&snapshot(), &args(None, None)).unwrap();
    assert!(change.configuration.peers[0].dial.is_none());
}

#[test]
fn import_upserts_the_dial_and_remove_deletes_the_source() {
    let peer = PeerIdentity::new(HubId::new(), [0x11; 32]).unwrap();
    let mut snapshot = snapshot();
    let change = import_change(
        &snapshot,
        PROFILE.into(),
        PathBuf::from(SOCKET),
        peer,
        false,
    )
    .unwrap()
    .unwrap();
    let joined = &change.configuration.peers[0];
    assert_eq!(joined.identity, peer);
    assert_eq!(joined.dial.as_ref().unwrap().ssh_target(), Some(PROFILE));
    assert!(joined.consume);
    snapshot.state.configuration = change.configuration;
    assert!(
        import_change(
            &snapshot,
            PROFILE.into(),
            PathBuf::from(SOCKET),
            peer,
            false
        )
        .unwrap()
        .is_none()
    );
    let removed = remove_change(&snapshot, peer.hub()).unwrap();
    assert!(removed.configuration.peers.is_empty());
    assert!(remove_change(&snapshot, HubId::new()).is_err());
}

#[test]
fn import_with_pin_only_does_not_consume() {
    let peer = PeerIdentity::new(HubId::new(), [0x22; 32]).unwrap();
    let change = import_change(
        &snapshot(),
        PROFILE.into(),
        PathBuf::from(SOCKET),
        peer,
        true,
    )
    .unwrap()
    .unwrap();
    assert!(!change.configuration.peers[0].consume);
}

#[test]
fn grant_change_upgrades_only_the_consumer_local_origin() {
    let consumer = HubId::new();
    let mut snapshot = snapshot();
    let peer = PeerIdentity::new(consumer, [0x33; 32]).unwrap();
    snapshot
        .state
        .configuration
        .peers
        .push(yolo_hub_federation_settings::Peer {
            identity: peer,
            dial: None,
            consume: false,
            exports: vec![yolo_hub_federation_settings::ExportPolicy {
                origin: snapshot.state.identity.hub(),
                rights: [yolo_hub_routing::Right::Observe].into_iter().collect(),
                remaining_reexports: 0,
            }],
        });
    let args = GrantMachine {
        consumer,
        public_key: "33".repeat(32),
        json: false,
    };
    let change = grant_change(&snapshot, &args).unwrap();
    let rights = &change.configuration.peers[0].exports[0].rights;
    let control: yolo_hub_routing::Rights = [
        yolo_hub_routing::Right::Observe,
        yolo_hub_routing::Right::TerminalRead,
        yolo_hub_routing::Right::TerminalWrite,
        yolo_hub_routing::Right::Lifecycle,
        yolo_hub_routing::Right::AgentControl,
        yolo_hub_routing::Right::ConversationRead,
    ]
    .into_iter()
    .collect();
    assert!(rights.allows(&control));
    assert!(
        !rights.allows(
            &[yolo_hub_routing::Right::WorkspaceWrite]
                .into_iter()
                .collect()
        )
    );
    assert_eq!(change.configuration.peers[0].identity, peer);
}
