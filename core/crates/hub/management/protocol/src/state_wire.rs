use crate::ConfigurationState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::Configuration;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StateWire {
    schema_version: u16,
    identity: PeerIdentity,
    revision: u64,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    last_change: Uuid,
    configuration: Configuration,
}

impl TryFrom<StateWire> for ConfigurationState {
    type Error = anyhow::Error;
    fn try_from(wire: StateWire) -> Result<Self, Self::Error> {
        let state = Self {
            schema_version: wire.schema_version,
            identity: wire.identity,
            revision: wire.revision,
            last_change: wire.last_change,
            configuration: wire.configuration,
        };
        state.validate()?;
        Ok(state)
    }
}

impl From<ConfigurationState> for StateWire {
    fn from(state: ConfigurationState) -> Self {
        Self {
            schema_version: state.schema_version,
            identity: state.identity,
            revision: state.revision,
            last_change: state.last_change,
            configuration: state.configuration,
        }
    }
}
