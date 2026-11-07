use crate::{Call, constants::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use yolo_hub_connector_protocol::{Command, ConnectorError, Reply, Response};
use yolo_hub_jsonrpc_protocol::{JSONRPC_VERSION, RpcError, PARSE_ERROR, INVALID_REQUEST, INVALID_PARAMS, METHOD_NOT_FOUND, INTERNAL_ERROR};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Answer(pub yolo_hub_jsonrpc_protocol::Answer);

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ErrorData {
    Connector { code: ConnectorError },
}

impl Answer {
    pub fn new(call: &Call, reply: Reply) -> Result<Self, ConnectorError> {
        let (result, error) = if let Reply::Error { code } = reply {
            (
                None,
                Some(RpcError {
                    code: CONNECTOR_ERROR,
                    message: code.to_string(),
                    data: Some(
                        serde_json::to_value(ErrorData::Connector { code })
                            .map_err(|_| ConnectorError::InvalidRequest)?,
                    ),
                }),
            )
        } else {
            let Value::Object(mut value) =
                serde_json::to_value(reply).map_err(|_| ConnectorError::InvalidRequest)?
            else {
                return Err(ConnectorError::InvalidRequest);
            };
            value.remove(RESULT_TAG);
            (Some(Value::Object(value)), None)
        };
        Ok(Self(yolo_hub_jsonrpc_protocol::Answer {
            jsonrpc: JSONRPC_VERSION.into(),
            id: Some(call.id.clone()),
            result,
            error,
        }))
    }

    pub fn reply(self, call: &Call) -> Result<Reply, ConnectorError> {
        if self.0.jsonrpc != JSONRPC_VERSION || self.0.id.as_ref() != Some(&call.id) {
            return Err(ConnectorError::InvalidRequest);
        }
        let reply = match (self.0.result, self.0.error) {
            (None, Some(RpcError { code: PARSE_ERROR | INVALID_REQUEST | INVALID_PARAMS | METHOD_NOT_FOUND, .. })) => Reply::Error { code: ConnectorError::InvalidRequest },
            (None, Some(RpcError { code: INTERNAL_ERROR, .. })) => Reply::Error { code: ConnectorError::OutcomeUnknown },
            (Some(Value::Object(mut result)), None) => {
                if result.contains_key(RESULT_TAG) {
                    return Err(ConnectorError::InvalidRequest);
                }
                let tag = match call.command {
                    Command::Hello {} => "hello",
                    Command::List {} => "enrollments",
                    Command::Register { .. } => "enrolled",
                    Command::Activate { .. } => "activated",
                    Command::Publish { .. } => "published",
                    Command::Revoke { .. } => "revoked",
                };
                result.insert(RESULT_TAG.into(), Value::String(tag.into()));
                serde_json::from_value(Value::Object(result))
                    .map_err(|_| ConnectorError::InvalidRequest)?
            }
            (
                None,
                Some(RpcError {
                    code: CONNECTOR_ERROR,
                    data: Some(data),
                    ..
                }),
            ) => {
                let ErrorData::Connector { code } =
                    serde_json::from_value(data).map_err(|_| ConnectorError::InvalidRequest)?;
                Reply::Error { code }
            }
            _ => return Err(ConnectorError::InvalidRequest),
        };
        Response::new(call, reply.clone()).validate_for(call)?;
        Ok(reply)
    }
}
