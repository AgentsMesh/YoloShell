use crate::{CONNECTOR_PROTOCOL_VERSION, Command, ConnectorError, validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::HubId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RequestWire")]
pub struct Request {
    pub version: u16,
    #[serde(with = "validation::id")]
    pub request_id: Uuid,
    pub hub: HubId,
    pub command: Command,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestWire {
    version: u16,
    #[serde(with = "validation::id")]
    request_id: Uuid,
    hub: HubId,
    command: Command,
}

impl Request {
    pub fn new(hub: HubId, command: Command) -> Result<Self, ConnectorError> {
        let request = Self {
            version: CONNECTOR_PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            hub,
            command,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), ConnectorError> {
        if self.version != CONNECTOR_PROTOCOL_VERSION {
            return Err(ConnectorError::UnsupportedSchema);
        }
        validation::request_id(self.request_id)?;
        self.command.validate_for(self.hub)
    }
}

impl TryFrom<RequestWire> for Request {
    type Error = ConnectorError;
    fn try_from(wire: RequestWire) -> Result<Self, Self::Error> {
        let value = Self {
            version: wire.version,
            request_id: wire.request_id,
            hub: wire.hub,
            command: wire.command,
        };
        value.validate()?;
        Ok(value)
    }
}
