use crate::{
    CONFIGURATION_SCHEMA, ConfigurationChange, ConfigurationState, FIRST_CONFIGURATION_REVISION,
};
use uuid::Uuid;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_federation_settings::{Configuration, Peer};
use yolo_hub_identity::HubId;

#[path = "constants.rs"]
pub mod constants;
use constants::*;

pub fn identity() -> PeerIdentity {
    PeerIdentity::new(
        HubId::from_uuid(Uuid::from_u128(HUB_VALUE)).unwrap(),
        [KEY_BYTE; ED25519_PUBLIC_KEY_BYTES],
    )
    .unwrap()
}
pub fn other_identity() -> PeerIdentity {
    PeerIdentity::new(
        HubId::from_uuid(Uuid::from_u128(OTHER_HUB_VALUE)).unwrap(),
        [OTHER_KEY_BYTE; ED25519_PUBLIC_KEY_BYTES],
    )
    .unwrap()
}
pub fn configuration() -> Configuration {
    Configuration {
        listen: None,
        peers: vec![Peer {
            identity: other_identity(),
            dial: None,
            consume: true,
            exports: vec![],
        }],
        trusted_origins: vec![],
        file_roots: vec![],
    }
}
pub fn state() -> ConfigurationState {
    ConfigurationState {
        schema_version: CONFIGURATION_SCHEMA,
        identity: identity(),
        revision: FIRST_CONFIGURATION_REVISION,
        last_change: Uuid::new_v4(),
        configuration: configuration(),
    }
}
pub fn change() -> ConfigurationChange {
    ConfigurationChange {
        request_id: Uuid::new_v4(),
        expected_revision: FIRST_CONFIGURATION_REVISION,
        configuration: configuration(),
    }
}
pub fn roundtrip<
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
>(
    value: &T,
) {
    let json = serde_json::to_vec(value).unwrap();
    assert_eq!(serde_json::from_slice::<T>(&json).unwrap(), *value);
}
pub fn reject<T: serde::Serialize + serde::de::DeserializeOwned>(
    value: &T,
    field: &str,
    replacement: serde_json::Value,
) {
    let mut wire = serde_json::to_value(value).unwrap();
    wire[field] = replacement;
    assert!(serde_json::from_value::<T>(wire).is_err());
}
