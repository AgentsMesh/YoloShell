use crate::{Fault, Id, constants::*};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Method, Request, RequestFields};
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Call {
    pub id: Id,
    request: Request,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireCall {
    jsonrpc: String,
    pub(super) id: Id,
    pub(super) method: String,
    pub(super) params: Map<String, Value>,
}

impl Call {
    /// Request notifications have no receipt or durable delivery contract. The
    /// Hub ignores them without executing an operation or sending a response.
    pub fn is_notification(value: &Value) -> bool {
        value.is_object()
            && value.get("id").is_none()
            && value.get("jsonrpc").and_then(Value::as_str) == Some(JSONRPC_VERSION)
            && value.get("method").is_some_and(Value::is_string)
    }
    pub fn new(destination: HubId, command: Command) -> Result<Self, PublicError> {
        Ok(Self::from_request(Request::new(destination, command)?))
    }
    pub fn from_request(request: Request) -> Self {
        Self {
            id: Id::fresh(),
            request,
        }
    }
    pub fn fields(&self) -> &RequestFields {
        self.request.fields()
    }
    pub fn request(&self) -> &Request {
        &self.request
    }

    pub fn parse(value: Value) -> Result<Self, Fault> {
        WireCall::parse(value)?.into_call()
    }
}

impl WireCall {
    pub(super) fn parse(value: Value) -> Result<Self, Fault> {
        let id = value
            .get("id")
            .cloned()
            .and_then(|value| Id::try_from(value).ok());
        let wire: Self =
            serde_json::from_value(value).map_err(|_| Fault::new(id.clone(), INVALID_REQUEST))?;
        if wire.jsonrpc != JSONRPC_VERSION {
            return Err(Fault::new(id, INVALID_REQUEST));
        }
        Ok(wire)
    }

    pub(super) fn into_call(self) -> Result<Call, Fault> {
        let id = Some(self.id.clone());
        let _: Method = serde_json::from_value(Value::String(self.method.clone()))
            .map_err(|_| Fault::new(id.clone(), METHOD_NOT_FOUND))?;
        let mut fields = self.params;
        let destination = fields
            .remove(DESTINATION_FIELD)
            .and_then(|value| serde_json::from_value(value).ok())
            .ok_or_else(|| Fault::new(id.clone(), INVALID_PARAMS))?;
        if fields.contains_key(METHOD_FIELD) {
            return Err(Fault::new(id, INVALID_PARAMS));
        }
        fields.insert(METHOD_FIELD.into(), Value::String(self.method));
        let command = serde_json::from_value(Value::Object(fields))
            .map_err(|_| Fault::new(id.clone(), INVALID_PARAMS))?;
        let request =
            Request::new(destination, command).map_err(|_| Fault::new(id, INVALID_PARAMS))?;
        Ok(Call {
            id: self.id,
            request,
        })
    }
}

impl Serialize for Call {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Value::Object(mut params) =
            serde_json::to_value(&self.fields().command).map_err(serde::ser::Error::custom)?
        else {
            unreachable!()
        };
        let Value::String(method) = params.remove(METHOD_FIELD).unwrap() else {
            unreachable!()
        };
        params.insert(
            DESTINATION_FIELD.into(),
            serde_json::to_value(self.fields().destination).map_err(serde::ser::Error::custom)?,
        );
        WireCall {
            jsonrpc: JSONRPC_VERSION.into(),
            id: self.id.clone(),
            method,
            params,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Call {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(Value::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::ops::Deref for Call {
    type Target = Request;
    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

impl PartialEq<Request> for Call {
    fn eq(&self, other: &Request) -> bool {
        self.request == *other
    }
}
