use crate::{Command, HUB_PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RequestFields", into = "RequestFields")]
pub struct Request(RequestFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestFields {
    pub version: u16,
    #[serde(with = "crate::id")]
    pub request_id: Uuid,
    pub destination: HubId,
    pub command: Command,
}

impl Request {
    pub fn new(destination: HubId, command: Command) -> Result<Self, PublicError> {
        let request_id = command.request_id().unwrap_or_else(Uuid::new_v4);
        RequestFields {
            version: HUB_PROTOCOL_VERSION,
            request_id,
            destination,
            command,
        }
        .try_into()
    }
    pub fn fields(&self) -> &RequestFields {
        &self.0
    }
}

impl TryFrom<RequestFields> for Request {
    type Error = PublicError;
    fn try_from(fields: RequestFields) -> Result<Self, Self::Error> {
        if fields.version != HUB_PROTOCOL_VERSION {
            return Err(PublicError::UnsupportedSchema);
        }
        if fields.request_id.is_nil()
            || fields
                .command
                .request_id()
                .is_some_and(|id| id != fields.request_id)
        {
            return Err(PublicError::InvalidRequest);
        }
        fields.command.validate_origin(fields.destination)?;
        Ok(Self(fields))
    }
}

impl From<Request> for RequestFields {
    fn from(value: Request) -> Self {
        value.0
    }
}
