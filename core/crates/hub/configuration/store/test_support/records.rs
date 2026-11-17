use super::constants::*;
use uuid::Uuid;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationChange, ConfigurationState, FIRST_CONFIGURATION_REVISION,
};

pub fn directory_identity() -> yolo_paths::identity::DurableObjectIdentity {
    let root = tempfile::tempdir().unwrap();
    yolo_paths::directory::PrivateDirectory::ensure(root.path())
        .unwrap()
        .durable_identity()
        .unwrap()
}

pub fn identity() -> PeerIdentity {
    PeerIdentity::new(HubId::new(), [LOCAL_KEY_BYTE; ED25519_PUBLIC_KEY_BYTES]).unwrap()
}

pub fn configuration() -> Configuration {
    Configuration {
        listen: None,
        peers: Vec::new(),
        file_roots: Vec::new(),
        trusted_origins: vec![
            PeerIdentity::new(HubId::new(), [OTHER_KEY_BYTE; ED25519_PUBLIC_KEY_BYTES]).unwrap(),
        ],
    }
}

pub fn state(identity: PeerIdentity) -> ConfigurationState {
    ConfigurationState {
        schema_version: CONFIGURATION_SCHEMA,
        identity,
        revision: FIRST_CONFIGURATION_REVISION,
        last_change: Uuid::new_v4(),
        configuration: Configuration::default(),
    }
}

pub fn change(current: &ConfigurationState) -> ConfigurationChange {
    ConfigurationChange {
        request_id: Uuid::new_v4(),
        expected_revision: current.revision,
        configuration: configuration(),
    }
}
