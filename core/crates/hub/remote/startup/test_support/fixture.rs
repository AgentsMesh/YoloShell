use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationSnapshot, ConfigurationState,
};
use yolo_hub_remote_startup_test_support::*;

pub fn snapshot() -> ConfigurationSnapshot {
    ConfigurationSnapshot {
        state: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity: peer(REMOTE_HUB, REMOTE_KEY),
            revision: FIRST_REVISION,
            last_change: uuid::Uuid::new_v4(),
            configuration: yolo_hub_federation_settings::Configuration::default(),
        },
        applied: true,
        peer_socket: Some("/run/user/1000/yolo/hub/peer.sock".into()),
    }
}
