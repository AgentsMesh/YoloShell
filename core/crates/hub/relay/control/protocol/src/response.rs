use crate::{Outcome, ProtocolError, Request, RequestFields, SCHEMA_VERSION};
use serde::{Serialize};
use uuid::Uuid;
use yolo_hub_agent_protocol::AgentControlRequest;
use yolo_hub_identity::RuntimeTarget;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResponseFields {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub launch_id: Uuid,
    pub terminal: RuntimeTarget,
    pub request: AgentControlRequest,
    pub outcome: Outcome,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ResponseFields")]
pub struct Response(ResponseFields);
impl Response {
    pub fn new(request: &Request, outcome: Outcome) -> Result<Self, ProtocolError> {
        let fields = request.fields();
        Self::try_from(ResponseFields {
            schema_version: SCHEMA_VERSION,
            launch_id: fields.launch_id,
            terminal: fields.terminal,
            request: fields.request.clone(),
            outcome,
        })
    }
    pub fn fields(&self) -> &ResponseFields {
        &self.0
    }
    pub fn validate(&self) -> Result<(), ProtocolError> {
        let request = self.echo()?;
        self.0.outcome.validate_for(&request)
    }
    pub fn validate_for(&self, request: &Request) -> Result<(), ProtocolError> {
        request.validate()?;
        self.validate()?;
        if self.echo()? != *request {
            return Err(ProtocolError::ResponseMismatch);
        }
        Ok(())
    }
    fn echo(&self) -> Result<Request, ProtocolError> {
        Request::new(RequestFields {
            schema_version: self.0.schema_version,
            launch_id: self.0.launch_id,
            terminal: self.0.terminal,
            request: self.0.request.clone(),
        })
    }
}
impl TryFrom<ResponseFields> for Response {
    type Error = ProtocolError;
    fn try_from(fields: ResponseFields) -> Result<Self, Self::Error> {
        let response = Self(fields);
        response.validate()?;
        Ok(response)
    }
}
impl From<Response> for ResponseFields {
    fn from(response: Response) -> Self {
        response.0
    }
}
