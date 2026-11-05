use super::{InvitePeer, invite_request, listen_change};
use crate::configuration::resolve_dial;
use crate::peer::provisioned_listener;
use std::net::SocketAddr;
use std::num::NonZeroU16;
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_enrollment_protocol::InviteDial;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationSnapshot, ConfigurationState,
};

mod constants;

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

fn invite() -> InvitePeer {
    InvitePeer {
        ttl_minutes: 10,
        no_export: false,
        import_peer: false,
        json: false,
    }
}

fn port(value: u16) -> Option<NonZeroU16> {
    NonZeroU16::new(value)
}

fn dial(host: &str, value: u16) -> InviteDial {
    InviteDial {
        host: host.into(),
        port: NonZeroU16::new(value).unwrap(),
    }
}

#[test]
fn an_invite_is_address_free() {
    let request = invite_request(&invite()).unwrap();
    assert_eq!(request.dial, None);
    assert!(request.policy.export_local);
    assert!(!request.policy.import_peer);
}

#[test]
fn invite_policy_and_lifetime_are_bounded() {
    let mut empty = invite();
    empty.no_export = true;
    assert!(invite_request(&empty).is_err());
    let mut short = invite();
    short.ttl_minutes = 0;
    assert!(invite_request(&short).is_err());
}

#[test]
fn an_inviter_without_a_listener_provisions_the_default() {
    assert_eq!(
        provisioned_listener(&snapshot(None, true)).unwrap(),
        Some(SocketAddr::from(([0, 0, 0, 0], 7400)))
    );
    let active = Some(SocketAddr::from(([0, 0, 0, 0], 7500)));
    assert_eq!(provisioned_listener(&snapshot(active, true)).unwrap(), None);
}

#[test]
fn a_persisted_listener_is_not_overwritten() {
    let mut pending = snapshot(None, false);
    pending.state.configuration.listen = Some(SocketAddr::from(([127, 0, 0, 1], 7400)));
    let error = provisioned_listener(&pending).unwrap_err().to_string();
    assert!(error.contains("not active"), "{error}");
    assert!(error.contains("127.0.0.1:7400"), "{error}");
}

#[test]
fn a_v2_code_requires_a_host_and_defaults_the_port() {
    let error = resolve_dial(None, None, None).unwrap_err().to_string();
    assert!(error.contains(constants::HOST_ARGUMENT), "{error}");
    assert_eq!(resolve_dial(None, Some("100.85.238.9"), None).unwrap(), dial("100.85.238.9", 7400));
    assert_eq!(
        resolve_dial(None, Some("100.85.238.9"), port(7500)).unwrap(),
        dial("100.85.238.9", 7500)
    );
}

#[test]
fn a_v1_code_keeps_its_embedded_address_and_accepts_overrides() {
    let embedded = dial("192.168.1.10", 7500);
    assert_eq!(
        resolve_dial(Some(&embedded), None, None).unwrap(),
        embedded.clone()
    );
    assert_eq!(
        resolve_dial(Some(&embedded), Some("100.85.238.9"), None).unwrap(),
        dial("100.85.238.9", 7500)
    );
    assert_eq!(
        resolve_dial(Some(&embedded), None, port(7400)).unwrap(),
        dial("192.168.1.10", 7400)
    );
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
