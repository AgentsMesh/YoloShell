use crate::model::{Entry, persistent};
#[path = "../test_support/constants.rs"]
mod constants;
use constants::*;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationState, FIRST_CONFIGURATION_REVISION,
};

fn entry() -> Entry {
    let identity = PeerIdentity::new(HubId::new(), [KEY_BYTE; ED25519_PUBLIC_KEY_BYTES]).unwrap();
    Entry {
        configuration: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity,
            revision: FIRST_CONFIGURATION_REVISION,
            last_change: uuid::Uuid::new_v4(),
            configuration: yolo_hub_federation_settings::Configuration {
                listen: Some(LOOPBACK.parse().unwrap()),
                ..Default::default()
            },
        },
        address: Some(ALLOCATED.parse().unwrap()),
    }
}

#[test]
fn endpoint_must_match_the_exact_requested_loopback_and_have_a_real_port() {
    let value = entry();
    let peer = value.configuration.identity;
    value.validate(peer).unwrap();
    value.ready().unwrap();
    for invalid in [LOOPBACK, WRONG_IP] {
        let mut changed = value.clone();
        changed.address = Some(invalid.parse().unwrap());
        assert!(changed.validate(peer).is_err());
    }
    let mut unassigned = value.clone();
    unassigned.address = None;
    unassigned.validate(peer).unwrap();
    assert!(unassigned.ready().is_err());
    let foreign =
        PeerIdentity::new(peer.hub(), [FOREIGN_KEY_BYTE; ED25519_PUBLIC_KEY_BYTES]).unwrap();
    assert!(value.validate(foreign).is_err());
}

#[test]
fn explicit_and_nonloopback_endpoints_never_use_an_ephemeral_binding() {
    for requested in [
        None,
        Some(ALLOCATED.parse().unwrap()),
        Some(WILDCARD.parse().unwrap()),
    ] {
        assert!(!persistent(requested));
        let mut value = entry();
        value.configuration.configuration.listen = requested;
        assert!(value.validate(value.configuration.identity).is_err());
        value.address = None;
        value.validate(value.configuration.identity).unwrap();
        value.ready().unwrap();
    }
}

#[test]
fn wire_requires_explicit_address_and_rejects_unknown_fields() {
    let value = entry();
    let encoded = serde_json::to_value(&value).unwrap();
    assert_eq!(
        serde_json::from_value::<Entry>(encoded.clone()).unwrap(),
        value
    );
    let mut missing = encoded.clone();
    missing.as_object_mut().unwrap().remove(ADDRESS_FIELD);
    assert!(serde_json::from_value::<Entry>(missing).is_err());
    let mut unknown = encoded;
    unknown[UNKNOWN_FIELD] = true.into();
    assert!(serde_json::from_value::<Entry>(unknown).is_err());
}
