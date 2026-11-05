use super::{InvitePeer, invite_request, listen_change};
use std::net::SocketAddr;
use std::num::NonZeroU16;
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationSnapshot, ConfigurationState,
};

fn snapshot(listen: Option<SocketAddr>, applied: bool) -> ConfigurationSnapshot {
    ConfigurationSnapshot {
        state: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity: PeerIdentity::new(HubId::new(), [1; 32]).unwrap(),
            revision: 1,
            last_change: Uuid::new_v4(),
            configuration: Configuration::default(),
        },
        applied,
        listen_address: listen,
    }
}

fn invite(host: Option<&str>, port: Option<u16>) -> InvitePeer {
    InvitePeer {
        host: host.map(str::to_owned),
        port: port.and_then(NonZeroU16::new),
        ttl_minutes: 10,
        no_export: false,
        import_peer: false,
        json: false,
    }
}

#[test]
fn a_host_without_a_listener_needs_a_port_and_names_the_remedy() {
    let error = invite_request(&snapshot(None, true), &invite(Some("100.85.238.9"), None))
        .unwrap_err()
        .to_string();
    assert!(error.contains("--port"), "{error}");
    assert!(error.contains("yolo hub listen"), "{error}");
}

#[test]
fn explicit_host_and_port_pair_even_without_a_listener() {
    let request = invite_request(
        &snapshot(None, true),
        &invite(Some("100.85.238.9"), Some(7400)),
    )
    .unwrap();
    assert_eq!(request.dial.host, "100.85.238.9");
    assert_eq!(request.dial.port.get(), 7400);
    assert!(request.policy.export_local);
    assert!(!request.policy.import_peer);
}

#[test]
fn an_applied_listener_supplies_the_default_port() {
    let listener = Some(SocketAddr::from(([0, 0, 0, 0], 7400)));
    let request = invite_request(
        &snapshot(listener, true),
        &invite(Some("100.85.238.9"), None),
    )
    .unwrap();
    assert_eq!(request.dial.host, "100.85.238.9");
    assert_eq!(request.dial.port.get(), 7400);
}

#[test]
fn an_unspecified_listener_still_requires_a_dialable_host() {
    let listener = Some(SocketAddr::from(([0, 0, 0, 0], 7400)));
    let error = invite_request(&snapshot(listener, true), &invite(None, None))
        .unwrap_err()
        .to_string();
    assert!(error.contains("--host"), "{error}");
    assert!(error.contains("current listener"), "{error}");
}

#[test]
fn listen_change_sets_the_listener_at_the_current_revision() {
    let address = SocketAddr::from(([127, 0, 0, 1], 7400));
    let change = listen_change(&snapshot(None, true), address).unwrap();
    assert_eq!(change.configuration.listen, Some(address));
    assert_eq!(change.expected_revision, 1);
}

#[test]
fn expiry_is_reported_in_human_units() {
    use crate::peer::expires_in_text;
    assert_eq!(
        expires_in_text(1_000, 1_000 + 9 * 60_000 + 58_000),
        "9m 58s"
    );
    assert_eq!(expires_in_text(1_000, 1_000 + 42_000), "42s");
    assert_eq!(expires_in_text(1_000, 1_000), "less than a minute");
}
