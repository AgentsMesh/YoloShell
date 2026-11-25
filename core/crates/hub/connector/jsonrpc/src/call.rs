use crate::constants::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use yolo_hub_connector_protocol::{Command, ConnectorError, Request};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{
    Fault, INVALID_PARAMS, INVALID_REQUEST, Id, JSONRPC_VERSION, METHOD_NOT_FOUND,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Call {
    pub id: Id,
    request: Request,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCall {
    jsonrpc: String,
    id: Id,
    method: String,
    params: Map<String, Value>,
}

impl Call {
    pub fn new(hub: HubId, command: Command) -> Result<Self, ConnectorError> {
        Ok(Self::from_request(Request::new(hub, command)?))
    }
    pub fn from_request(request: Request) -> Self {
        Self {
            id: Id::fresh(),
            request,
        }
    }
    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn parse(value: Value) -> Result<Self, Fault> {
        let id = value
            .get("id")
            .cloned()
            .and_then(|value| Id::try_from(value).ok());
        let wire: WireCall =
            serde_json::from_value(value).map_err(|_| Fault::new(id.clone(), INVALID_REQUEST))?;
        if wire.jsonrpc != JSONRPC_VERSION {
            return Err(Fault::new(id, INVALID_REQUEST));
        }
        let method = wire
            .method
            .strip_prefix(METHOD_PREFIX)
            .filter(|method| METHODS.contains(method))
            .ok_or_else(|| Fault::new(id.clone(), METHOD_NOT_FOUND))?;
        let mut params = wire.params;
        let operation_id = params
            .remove(OPERATION_ID_FIELD)
            .and_then(|value| serde_json::from_value(value).ok())
            .ok_or_else(|| Fault::new(id.clone(), INVALID_PARAMS))?;
        let hub = params
            .remove(DESTINATION_FIELD)
            .and_then(|value| serde_json::from_value(value).ok())
            .ok_or_else(|| Fault::new(id.clone(), INVALID_PARAMS))?;
        if params.contains_key(METHOD_FIELD) {
            return Err(Fault::new(id, INVALID_PARAMS));
        }
        params.insert(METHOD_FIELD.into(), Value::String(method.into()));
        let command = serde_json::from_value(Value::Object(params))
            .map_err(|_| Fault::new(id.clone(), INVALID_PARAMS))?;
        let mut request =
            Request::new(hub, command).map_err(|_| Fault::new(id.clone(), INVALID_PARAMS))?;
        request.request_id = operation_id;
        request
            .validate()
            .map_err(|_| Fault::new(id, INVALID_PARAMS))?;
        Ok(Self {
            id: wire.id,
            request,
        })
    }
}

impl Serialize for Call {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Value::Object(mut params) =
            serde_json::to_value(&self.command).map_err(serde::ser::Error::custom)?
        else {
            unreachable!()
        };
        let Value::String(method) = params.remove(METHOD_FIELD).unwrap() else {
            unreachable!()
        };
        params.insert(
            DESTINATION_FIELD.into(),
            serde_json::to_value(self.hub).map_err(serde::ser::Error::custom)?,
        );
        params.insert(
            OPERATION_ID_FIELD.into(),
            serde_json::to_value(self.request_id).map_err(serde::ser::Error::custom)?,
        );
        WireCall {
            jsonrpc: JSONRPC_VERSION.into(),
            id: self.id.clone(),
            method: format!("{METHOD_PREFIX}{method}"),
            params,
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Call {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        Self::parse(Value::deserialize(de)?).map_err(serde::de::Error::custom)
    }
}
impl std::ops::Deref for Call {
    type Target = Request;
    fn deref(&self) -> &Request {
        &self.request
    }
}
impl PartialEq<Request> for Call {
    fn eq(&self, request: &Request) -> bool {
        &self.request == request
    }
}
