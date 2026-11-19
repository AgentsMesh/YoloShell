use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use uuid::Uuid;
use yolo_hub_rpc_protocol::{Command, Hello, RPC_REQUEST_ID_FIELD, Reply, Request, Response};

fn roundtrip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let bytes = serde_json::to_vec(value).unwrap();
    assert_eq!(&serde_json::from_slice::<T>(&bytes).unwrap(), value);
}

#[test]
fn all_commands_and_replies_roundtrip_without_unknown_fields() {
    for (command, reply) in samples() {
        roundtrip(&command);
        roundtrip(&reply);
        let request = request(command);
        roundtrip(&request);
        let response = Response::new(&request, hub(HUB_ID), reply);
        roundtrip(&response);
        let mut envelope = serde_json::to_value(&request).unwrap();
        envelope[UNKNOWN_FIELD] = Value::Bool(true);
        assert!(serde_json::from_value::<Request>(envelope).is_err());
        let mut envelope = serde_json::to_value(&response).unwrap();
        envelope[UNKNOWN_FIELD] = Value::Bool(true);
        assert!(serde_json::from_value::<Response>(envelope).is_err());
        let mut body = serde_json::to_value(&request.fields().command).unwrap();
        body[UNKNOWN_FIELD] = Value::Bool(true);
        assert!(serde_json::from_value::<Command>(body).is_err());
        let mut body = serde_json::to_value(&response.reply).unwrap();
        body[UNKNOWN_FIELD] = Value::Bool(true);
        assert!(serde_json::from_value::<Reply>(body).is_err());
    }
}

#[test]
fn correlation_ids_require_non_nil_canonical_lowercase_hyphenated_uuid() {
    let request = request(Command::Input { request: input() });
    let response = Response::new(&request, hub(HUB_ID), Reply::Input { duplicate: false });
    let invalid = [
        Value::String(id(REQUEST_ID).to_string().to_uppercase()),
        Value::String(id(REQUEST_ID).simple().to_string()),
        Value::String(Uuid::nil().to_string()),
        Value::String(UNKNOWN_TEXT.into()),
        Value::Bool(true),
    ];
    for replacement in invalid {
        let mut wire = serde_json::to_value(&request).unwrap();
        wire[RPC_REQUEST_ID_FIELD] = replacement.clone();
        assert!(serde_json::from_value::<Request>(wire).is_err());
        let mut wire = serde_json::to_value(&response).unwrap();
        wire[RPC_REQUEST_ID_FIELD] = replacement;
        assert!(serde_json::from_value::<Response>(wire).is_err());
    }
    let mut invalid_response = response;
    invalid_response.request_id = Uuid::nil();
    assert!(serde_json::to_vec(&invalid_response).is_err());
}

#[test]
fn finalization_token_is_canonical_and_hello_rejects_extra_fields() {
    let original = request(Command::Finalize {
        creation_token: id(REQUEST_ID),
        creation: creation(),
    });
    for token in [
        id(REQUEST_ID).simple().to_string(),
        id(REQUEST_ID).to_string().to_uppercase(),
        Uuid::nil().to_string(),
    ] {
        let mut wire = serde_json::to_value(&original).unwrap();
        wire[COMMAND_FIELD][CREATION_TOKEN_FIELD] = Value::String(token);
        assert!(serde_json::from_value::<Request>(wire).is_err());
    }
    let invalid = Command::Finalize {
        creation_token: Uuid::nil(),
        creation: creation(),
    };
    assert!(serde_json::to_vec(&invalid).is_err());
    let hello = samples()
        .into_iter()
        .find_map(|(_, reply)| match reply {
            Reply::Hello { hello } => Some(hello),
            _ => None,
        })
        .unwrap();
    let mut wire = serde_json::to_value(hello).unwrap();
    wire[UNKNOWN_FIELD] = Value::Bool(true);
    assert!(serde_json::from_value::<Hello>(wire).is_err());
}
