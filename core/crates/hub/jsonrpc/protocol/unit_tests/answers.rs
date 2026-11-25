use serde_json::json;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::*;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Reply};
use yolo_hub_terminal_protocol::PublicError;

fn call() -> Call {
    Call::new(HubId::new(), Command::Hello {}).unwrap()
}
fn answer(call: &Call) -> Answer {
    Answer::new(
        call,
        Reply::Hello {
            hello: Hello {
                hub_id: call.fields().destination,
                version: HUB_PROTOCOL_VERSION,
                methods: vec![],
            },
        },
    )
    .unwrap()
}

#[test]
fn success_error_and_invalid_combinations_have_exact_jsonrpc_semantics() {
    let call = call();
    let valid = answer(&call);
    let wire = serde_json::to_value(&valid).unwrap();
    assert!(wire.get("error").is_none());
    assert!(wire["result"].get("result").is_none());
    assert!(matches!(
        valid.clone().reply(&call),
        Ok(Reply::Hello { .. })
    ));
    let failure = Answer::new(
        &call,
        Reply::Error {
            code: PublicError::Unauthorized,
        },
    )
    .unwrap();
    assert_eq!(failure.error.as_ref().unwrap().code, HUB_ERROR);
    assert!(
        serde_json::to_value(&failure)
            .unwrap()
            .get("result")
            .is_none()
    );
    assert_eq!(
        failure.clone().reply(&call),
        Ok(Reply::Error {
            code: PublicError::Unauthorized
        })
    );
    let mut malformed = valid.clone();
    malformed.error = failure.error;
    assert!(malformed.reply(&call).is_err());
    let mut malformed = valid.clone();
    malformed.result = None;
    assert!(malformed.reply(&call).is_err());
    let mut malformed = valid.clone();
    malformed.jsonrpc.clear();
    assert!(malformed.reply(&call).is_err());
    let mut malformed = valid.clone();
    malformed.id = None;
    assert!(malformed.reply(&call).is_err());
    let mut malformed = valid.clone();
    malformed.id = Some(Id::fresh());
    assert!(malformed.reply(&call).is_err());
    for value in [
        json!(true),
        json!({"result":"hello"}),
        json!({}),
        json!({"unknown":true}),
    ] {
        let mut malformed = valid.clone();
        malformed.result = Some(value);
        assert!(malformed.reply(&call).is_err());
    }
}

#[test]
fn protocol_faults_are_bounded_descriptions_not_domain_rejections() {
    for code in [
        PARSE_ERROR,
        INVALID_REQUEST,
        METHOD_NOT_FOUND,
        INVALID_PARAMS,
        INTERNAL_ERROR,
    ] {
        let fault = Fault::new(None, code);
        assert!(!fault.to_string().is_empty());
        let answer = Answer::failure(fault);
        assert!(answer.id.is_none() && answer.result.is_none());
        assert_eq!(answer.error.as_ref().unwrap().code, code);
        assert!(answer.reply(&call()).is_err());
    }
    let call = call();
    for data in [
        None,
        Some(json!({})),
        Some(json!({"kind":"hub","code":"unauthorized"})),
    ] {
        let answer = Answer {
            jsonrpc: JSONRPC_VERSION.into(),
            id: Some(call.id.clone()),
            result: None,
            error: Some(RpcError {
                code: CONVERSATION_ERROR,
                message: "failure".into(),
                data,
            }),
        };
        assert!(answer.reply(&call).is_err());
    }
}

#[test]
fn null_result_cannot_hide_an_extra_outcome_and_response_id_is_required() {
    let call = call();
    let mut value = serde_json::to_value(
        Answer::new(
            &call,
            Reply::Error {
                code: PublicError::Unauthorized,
            },
        )
        .unwrap(),
    )
    .unwrap();
    value["result"] = serde_json::Value::Null;
    let decoded: Answer = serde_json::from_value(value).unwrap();
    assert!(decoded.reply(&call).is_err());
    let mut value = serde_json::to_value(answer(&call)).unwrap();
    value.as_object_mut().unwrap().remove("id");
    assert!(serde_json::from_value::<Answer>(value).is_err());
}
