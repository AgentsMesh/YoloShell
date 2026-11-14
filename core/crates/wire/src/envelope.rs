use super::{
    EVENT_KIND, PROTOCOL_VERSION, ProtocolError, REQUEST_KIND, RESPONSE_KIND, error_code,
    from_value, to_value,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use yolo_capnp_value::Value;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub version: u16,
    pub kind: String,
    pub request_id: String,
    pub method: String,
    pub payload: Value,
}

impl RequestEnvelope {
    pub fn new(request_id: impl Into<String>, method: impl Into<String>, payload: Value) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: REQUEST_KIND.to_string(),
            request_id: request_id.into(),
            method: method.into(),
            payload,
        }
    }

    pub fn decode_payload<T: DeserializeOwned>(&self) -> Result<T, ProtocolError> {
        from_value(self.payload.clone())
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub version: u16,
    pub kind: String,
    pub request_id: String,
    pub method: String,
    pub ok: bool,
    pub payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}

impl ResponseEnvelope {
    pub fn success<T: Serialize>(
        request: &RequestEnvelope,
        payload: &T,
    ) -> Result<Self, ProtocolError> {
        Ok(Self {
            version: PROTOCOL_VERSION,
            kind: RESPONSE_KIND.to_string(),
            request_id: request.request_id.clone(),
            method: request.method.clone(),
            ok: true,
            payload: to_value(payload)?,
            error: None,
        })
    }

    pub fn empty(request: &RequestEnvelope) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: RESPONSE_KIND.to_string(),
            request_id: request.request_id.clone(),
            method: request.method.clone(),
            ok: true,
            payload: Value::Null,
            error: None,
        }
    }

    pub fn failure(request: &RequestEnvelope, error: ErrorBody) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: RESPONSE_KIND.to_string(),
            request_id: request.request_id.clone(),
            method: request.method.clone(),
            ok: false,
            payload: Value::Null,
            error: Some(error),
        }
    }

    pub fn decode_payload<T: DeserializeOwned>(&self) -> Result<T, ProtocolError> {
        from_value(self.payload.clone())
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    pub version: u16,
    pub kind: String,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_epoch: Option<String>,
    pub payload: Value,
}

impl EventEnvelope {
    pub fn new<T: Serialize>(
        event: impl Into<String>,
        terminal_id: Option<String>,
        runtime_epoch: Option<String>,
        payload: &T,
    ) -> Result<Self, ProtocolError> {
        Ok(Self {
            version: PROTOCOL_VERSION,
            kind: EVENT_KIND.to_string(),
            event: event.into(),
            terminal_id,
            runtime_epoch,
            payload: to_value(payload)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorBody {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(error_code::INVALID_REQUEST, message, false)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(error_code::NOT_FOUND, message, false)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(error_code::CONFLICT, message, false)
    }

    pub fn worker_shutting_down(message: impl Into<String>) -> Self {
        Self::new(error_code::WORKER_SHUTTING_DOWN, message, true)
    }

    pub fn protocol_incompatible(message: impl Into<String>) -> Self {
        Self::new(error_code::PROTOCOL_INCOMPATIBLE, message, false)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(error_code::INTERNAL, message, true)
    }

    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
            details: None,
        }
    }

    pub fn lease(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
            details: Some(details),
        }
    }
}

include!("envelope_debug.rs");
