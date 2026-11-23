use serde_json::json;
use yolo_hub_connector_jsonrpc::{Answer, CONNECTOR_ERROR, Call};
use yolo_hub_connector_protocol::{Command, ConnectorError, Reply, Request};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND};
mod constants;
use constants::OPERATION_ID_FIELD;

#[test]
fn observation_plugin_methods_keep_business_identity_independent_from_rpc_correlation() {
    let request = Request::new(HubId::new(), Command::Hello {}).unwrap();
    let call = Call::from_request(request.clone());
    assert_eq!(call.request(), &request);
    let wire = serde_json::to_value(&call).unwrap();
    assert_eq!(wire["method"], "connector.hello");
    assert_eq!(
        wire["params"][OPERATION_ID_FIELD],
        request.request_id.to_string()
    );
    assert_ne!(wire["id"], wire["params"][OPERATION_ID_FIELD]);
    let received = Call::parse(wire).unwrap();
    assert_eq!(received, call);
    let reply = Reply::Hello { hub: request.hub };
    let answer = Answer::new(&received, reply.clone()).unwrap();
    assert_eq!(answer.reply(&call).unwrap(), reply);
}

#[test]
fn connector_errors_are_typed_and_unknown_methods_cannot_reach_the_registry() {
    let call = Call::new(HubId::new(), Command::Hello {}).unwrap();
    let rejection = Answer::new(
        &call,
        Reply::Error {
            code: ConnectorError::Unauthorized,
        },
    )
    .unwrap();
    assert_eq!(rejection.0.error.as_ref().unwrap().code, CONNECTOR_ERROR);
    assert_eq!(
        rejection.reply(&call).unwrap(),
        Reply::Error {
            code: ConnectorError::Unauthorized
        }
    );
    let valid = serde_json::to_value(&call).unwrap();
    for (field, value, code) in [
        ("jsonrpc", json!("1.0"), INVALID_REQUEST),
        ("id", json!(null), INVALID_REQUEST),
        ("method", json!("hello"), METHOD_NOT_FOUND),
        ("method", json!("connector.unknown"), METHOD_NOT_FOUND),
        ("params", json!({}), INVALID_PARAMS),
    ] {
        let mut value_wire = valid.clone();
        value_wire[field] = value;
        assert_eq!(Call::parse(value_wire).unwrap_err().error.code, code);
    }
    for extra in ["method", "authority", "custody"] {
        let mut value = valid.clone();
        value["params"][extra] = json!(true);
        assert_eq!(Call::parse(value).unwrap_err().error.code, INVALID_PARAMS);
    }
    let mut value = valid;
    value["params"][OPERATION_ID_FIELD] = json!("00000000-0000-0000-0000-000000000000");
    assert_eq!(Call::parse(value).unwrap_err().error.code, INVALID_PARAMS);
}

#[test]
fn wrong_correlation_origin_and_result_shape_never_acknowledge_an_observation() {
    let call = Call::new(HubId::new(), Command::Hello {}).unwrap();
    let valid = Answer::new(&call, Reply::Hello { hub: call.hub }).unwrap();
    let mut invalid = valid.clone();
    invalid.0.id = None;
    assert!(invalid.reply(&call).is_err());
    let mut invalid = valid.clone();
    invalid.0.jsonrpc.clear();
    assert!(invalid.reply(&call).is_err());
    for body in [
        json!(null),
        json!({}),
        json!({"result":"hello"}),
        json!({"hub":HubId::new()}),
    ] {
        let mut invalid = valid.clone();
        invalid.0.result = Some(body);
        assert!(invalid.reply(&call).is_err());
    }
    let mut invalid = valid;
    invalid.0.error = Answer::new(
        &call,
        Reply::Error {
            code: ConnectorError::InvalidRequest,
        },
    )
    .unwrap()
    .0
    .error;
    assert!(invalid.reply(&call).is_err());
}
