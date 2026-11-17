use crate::{Id, constants::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("{error:?}")]
pub struct Fault {
    pub id: Option<Id>,
    pub error: RpcError,
}

impl Fault {
    pub fn new(id: Option<Id>, code: i32) -> Self {
        let message = match code {
            PARSE_ERROR => "Parse error",
            INVALID_REQUEST => "Invalid Request",
            METHOD_NOT_FOUND => "Method not found",
            INVALID_PARAMS => "Invalid params",
            _ => "Internal error",
        };
        Self {
            id,
            error: RpcError {
                code,
                message: message.into(),
                data: None,
            },
        }
    }
}
