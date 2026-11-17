use crate::{Call, Fault, Id, JSONRPC_VERSION, RpcError, result};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use yolo_hub_rpc_protocol::{Reply, Response};
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub jsonrpc: String,
    #[serde(deserialize_with = "required_id")]
    pub id: Option<Id>,
    #[serde(
        default,
        deserialize_with = "present_result",
        skip_serializing_if = "Option::is_none"
    )]
    pub result: Option<Value>,
    #[serde(
        default,
        deserialize_with = "present_error",
        skip_serializing_if = "Option::is_none"
    )]
    pub error: Option<RpcError>,
}

impl std::fmt::Debug for Answer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Answer")
            .field("jsonrpc", &self.jsonrpc)
            .field("id", &self.id)
            .field("has_result", &self.result.is_some())
            .field("has_error", &self.error.is_some())
            .finish()
    }
}

fn required_id<'de, D: Deserializer<'de>>(de: D) -> Result<Option<Id>, D::Error> {
    Option::<Id>::deserialize(de)
}
fn present_result<'de, D: Deserializer<'de>>(de: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(de).map(Some)
}
fn present_error<'de, D: Deserializer<'de>>(de: D) -> Result<Option<RpcError>, D::Error> {
    RpcError::deserialize(de).map(Some)
}

impl Answer {
    pub fn new(call: &Call, reply: Reply) -> Result<Self, PublicError> {
        let (result, error) = result::encode(reply)?;
        Ok(Self {
            jsonrpc: JSONRPC_VERSION.into(),
            id: Some(call.id.clone()),
            result,
            error,
        })
    }
    pub fn failure(fault: Fault) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.into(),
            id: fault.id,
            result: None,
            error: Some(fault.error),
        }
    }
    pub fn reply(self, call: &Call) -> Result<Reply, PublicError> {
        if self.jsonrpc != JSONRPC_VERSION || self.id.as_ref() != Some(&call.id) {
            return Err(PublicError::InvalidRequest);
        }
        let reply = match (self.result, self.error) {
            (Some(value), None) => result::decode(value, call.fields().command.method())?,
            (None, Some(error)) => result::decode_error(error)?,
            _ => return Err(PublicError::InvalidPayload),
        };
        Response::new(call, call.fields().destination, reply.clone()).validate_for(call)?;
        Ok(reply)
    }
}
