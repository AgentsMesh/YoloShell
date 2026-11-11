use crate::{ProtocolError, SCHEMA_VERSION};
use serde::{Serialize};
use uuid::Uuid;
use yolo_hub_agent_protocol::{AgentAction, AgentControlRequest};
use yolo_hub_identity::RuntimeTarget;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RequestFields {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub launch_id: Uuid,
    pub terminal: RuntimeTarget,
    pub request: AgentControlRequest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "RequestFields")]
pub struct Request(RequestFields);
impl Request {
    pub fn new(fields: RequestFields) -> Result<Self, ProtocolError> {
        let request = Self(fields);
        request.validate()?;
        Ok(request)
    }
    pub fn fields(&self) -> &RequestFields {
        &self.0
    }
    pub fn validate(&self) -> Result<(), ProtocolError> {
        let fields = &self.0;
        if fields.schema_version != SCHEMA_VERSION {
            return Err(ProtocolError::UnsupportedSchema);
        }
        if fields.launch_id.is_nil() {
            return Err(ProtocolError::InvalidIdentity);
        }
        if !matches!(
            fields.request.fields().action,
            AgentAction::Interrupt {} | AgentAction::SubmitPrompt { .. }
        ) {
            return Err(ProtocolError::UnsupportedAction);
        }
        if fields.request.fields().target.address.hub_id != fields.terminal.address.hub_id {
            return Err(ProtocolError::OriginMismatch);
        }
        Ok(())
    }
}
impl TryFrom<RequestFields> for Request {
    type Error = ProtocolError;
    fn try_from(fields: RequestFields) -> Result<Self, Self::Error> {
        Self::new(fields)
    }
}
impl From<Request> for RequestFields {
    fn from(request: Request) -> Self {
        request.0
    }
}
