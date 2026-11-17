use serde_json::{Value, json};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::*;
use yolo_hub_rpc_protocol::{Command, Request};

#[test]
fn method_and_routing_are_public_fields_without_a_private_envelope() {
    let domain = Request::new(
        HubId::new(),
        Command::List {
            include_archived: false,
            after: None,
        },
    )
    .unwrap();
    let call = Call::from_request(domain.clone());
    let value = serde_json::to_value(&call).unwrap();
    assert_eq!(value["jsonrpc"], JSONRPC_VERSION);
    assert_eq!(value["method"], "list");
    assert_eq!(
        value["params"]["destination"],
        serde_json::to_value(domain.fields().destination).unwrap()
    );
    assert!(value.get("command").is_none());
    assert!(value.get("version").is_none());
    assert_ne!(value["id"], domain.fields().request_id.to_string());
    let decoded = Call::parse(value).unwrap();
    assert_eq!(decoded.id, call.id);
    assert_eq!(decoded.fields().command, domain.fields().command);
    assert_eq!(
        decoded.request().fields().destination,
        domain.fields().destination
    );
}

#[test]
fn invalid_requests_never_reach_command_dispatch() {
    let call = Call::new(HubId::new(), Command::Hello {}).unwrap();
    let valid = serde_json::to_value(call).unwrap();
    for (field, value, expected) in [
        ("jsonrpc", json!("1.0"), INVALID_REQUEST),
        ("id", Value::Null, INVALID_REQUEST),
        ("id", json!(1.5), INVALID_REQUEST),
        ("id", json!(""), INVALID_REQUEST),
        ("id", json!("x".repeat(MAX_ID_BYTES + 1)), INVALID_REQUEST),
        ("method", json!("call"), METHOD_NOT_FOUND),
        ("params", json!([]), INVALID_REQUEST),
        ("params", json!({}), INVALID_PARAMS),
        ("extra", json!(true), INVALID_REQUEST),
    ] {
        let mut malformed = valid.clone();
        malformed[field] = value;
        assert_eq!(Call::parse(malformed).unwrap_err().error.code, expected);
    }
    for field in ["id", "method", "params", "jsonrpc"] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert_eq!(
            Call::parse(missing).unwrap_err().error.code,
            INVALID_REQUEST
        );
    }
    for extra in ["method", "unknown"] {
        let mut malformed = valid.clone();
        malformed["params"][extra] = json!("hello");
        assert_eq!(
            Call::parse(malformed).unwrap_err().error.code,
            INVALID_PARAMS
        );
    }
    assert_eq!(
        Call::parse(json!([valid])).unwrap_err().error.code,
        INVALID_REQUEST
    );
}

#[test]
fn transport_ids_support_strings_and_integers_and_preserve_their_types() {
    for value in [
        json!("external-client"),
        json!(0),
        json!(-1),
        json!(u64::MAX),
    ] {
        let id = Id::try_from(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(id.clone()).unwrap(), value);
        assert_eq!(Value::from(id), value);
    }
    for value in [Value::Null, json!(false), json!([]), json!({}), json!(2.0)] {
        assert!(Id::try_from(value).is_err());
    }
}
