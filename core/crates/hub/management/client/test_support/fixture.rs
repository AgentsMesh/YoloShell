use uuid::Uuid;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::*;
use yolo_hub_rpc_protocol::Command;

pub fn snapshot() -> ConfigurationSnapshot {
    const KEY: u8 = 1;
    ConfigurationSnapshot {
        state: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity: PeerIdentity::new(HubId::new(), [KEY; ED25519_PUBLIC_KEY_BYTES]).unwrap(),
            revision: FIRST_CONFIGURATION_REVISION,
            last_change: Uuid::new_v4(),
            configuration: Default::default(),
        },
        applied: true,
        listen_address: None,
    }
}
pub fn change(state: &ConfigurationSnapshot) -> (Command, ConfigurationSnapshot) {
    let change = ConfigurationChange {
        request_id: Uuid::new_v4(),
        expected_revision: state.state.revision,
        configuration: state.state.configuration.clone(),
    };
    let mut result = state.clone();
    result.state.revision += CONFIGURATION_REVISION_STEP;
    result.state.last_change = change.request_id;
    (Command::Configure { change }, result)
}
