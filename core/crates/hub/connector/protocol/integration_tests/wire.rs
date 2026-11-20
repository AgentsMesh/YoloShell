use crate::{constants::*, fixture::Fixture};
use yolo_hub_connector_jsonrpc::{Answer, Call};
use yolo_hub_connector_protocol::*;
use yolo_hub_jsonrpc_protocol::decode;
use yolo_hub_jsonrpc_transport::encode;

#[test]
fn every_connector_exchange_uses_strict_public_jsonrpc_transport() {
    let fixture = Fixture::new();
    for (request, reply) in fixture.samples() {
        let request = Call::from_request(request);
        let wire = encode(&request).unwrap();
        let received: Call = decode(&wire).unwrap();
        assert_eq!(received, request);
        let response = Answer::new(&received, reply).unwrap();
        let wire = encode(&response).unwrap();
        let received: Answer = decode(&wire).unwrap();
        received.clone().reply(&request).unwrap();
        assert_eq!(received, response);
    }
}

#[test]
fn unknown_fields_nil_ids_wrong_schema_and_forged_observation_authority_fail_at_ingress() {
    let fixture = Fixture::new();
    for (request, reply) in fixture.samples() {
        let request = Call::from_request(request);
        let mut value = serde_json::to_value(&request).unwrap();
        value[UNKNOWN_FIELD] = true.into();
        assert!(decode::<Call>(&encode(&value).unwrap()).is_err());
        let response = Answer::new(&request, reply).unwrap();
        let mut value = serde_json::to_value(&response).unwrap();
        value["result"][UNKNOWN_FIELD] = true.into();
        assert!(
            decode::<Answer>(&encode(&value).unwrap())
                .unwrap()
                .reply(&request)
                .is_err()
        );
        let mut value = serde_json::to_value(&response).unwrap();
        value["id"] = uuid::Uuid::nil().to_string().into();
        assert!(
            decode::<Answer>(&encode(&value).unwrap())
                .unwrap()
                .reply(&request)
                .is_err()
        );
        let mut value = serde_json::to_value(&response).unwrap();
        value["jsonrpc"] = "1.0".into();
        assert!(
            decode::<Answer>(&encode(&value).unwrap())
                .unwrap()
                .reply(&request)
                .is_err()
        );
    }
    let request = Call::new(
        fixture.hub,
        Command::Publish {
            publication: fixture.publication,
        },
    )
    .unwrap();
    for injected in FORBIDDEN_STATE_FIELDS {
        let mut value = serde_json::to_value(&request).unwrap();
        value["params"][field::PUBLICATION][field::STATE][*injected] = true.into();
        assert!(decode::<Call>(&encode(&value).unwrap()).is_err());
    }
}

#[test]
fn serializing_secret_receipt_is_intentional_but_debug_does_not_disclose_credentials() {
    let fixture = Fixture::new();
    let secret = String::from(fixture.enrollment.credential);
    let request = Call::new(
        fixture.hub,
        Command::Register {
            scope: fixture.enrollment.scope.clone(),
        },
    )
    .unwrap();
    let response = Answer::new(
        &request,
        Reply::Enrolled {
            enrollment: fixture.enrollment,
        },
    )
    .unwrap();
    let encoded = serde_json::to_string(&response).unwrap();
    assert!(encoded.contains(&secret));
    assert!(!format!("{response:#?}").contains(&secret));
    let decoded = serde_json::from_str::<Answer>(&encoded).unwrap();
    decoded.reply(&request).unwrap();
}
