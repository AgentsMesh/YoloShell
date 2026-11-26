use serde_json::{Value, json};
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_rpc_protocol_test_support::{commands, fixtures};
use yolo_hub_terminal_protocol::PublicError;

use super::constants::RESULT;
const ERROR: &str = "error";
const UNKNOWN_FIELD: &str = "untrusted";
const SECRET: &str = "secret-terminal-output-must-not-appear-in-debug";

#[test]
fn every_success_family_rejects_ambiguous_outcomes_injected_tags_and_unknown_body_fields() {
    for (command, reply) in commands::samples() {
        let call = Call::from_request(commands::request(command));
        if matches!(reply, Reply::Attached { .. }) {
            assert_eq!(Answer::new(&call, reply), Err(PublicError::InvalidPayload));
            continue;
        }
        if matches!(reply, Reply::ConversationUnavailable { .. }) {
            continue;
        }
        let valid = Answer::new(&call, reply.clone())
            .unwrap_or_else(|error| panic!("{:?}: {error:?}", call.fields().command.method()));
        assert_eq!(valid.clone().reply(&call).unwrap(), reply);
        let body = valid.result.as_ref().unwrap();
        for poison in [RESULT, UNKNOWN_FIELD] {
            let mut forged = valid.clone();
            forged.result.as_mut().unwrap()[poison] = json!(SECRET);
            assert!(
                forged.reply(&call).is_err(),
                "{:?}",
                call.fields().command.method()
            );
        }
        for non_object in [Value::Null, json!([]), json!(true)] {
            let mut forged = valid.clone();
            forged.result = Some(non_object);
            assert!(forged.reply(&call).is_err());
        }
        let mut wire = serde_json::to_value(&valid).unwrap();
        wire[UNKNOWN_FIELD] = json!(SECRET);
        assert!(serde_json::from_value::<Answer>(wire).is_err());
        let mut wire = serde_json::to_value(&valid).unwrap();
        wire[ERROR] = Value::Null;
        assert!(serde_json::from_value::<Answer>(wire).is_err());
        assert!(body.get(RESULT).is_none());
    }
}

#[test]
fn result_shape_and_exact_terminal_identity_remain_bound_to_the_pending_call() {
    let call = Call::new(
        fixtures::target().address.hub_id,
        Command::Get {
            address: fixtures::target().address,
        },
    )
    .unwrap();
    let foreign = fixtures::summary(
        fixtures::constants::HUB_ID,
        fixtures::constants::SECOND_SESSION_ID,
    );
    let answer = Answer::new(&call, Reply::Terminal { terminal: foreign }).unwrap();
    assert_eq!(answer.reply(&call), Err(PublicError::InvalidPayload));
    let grant = fixtures::grant();
    let input = Call::new(
        fixtures::target().address.hub_id,
        Command::Input {
            request: fixtures::input(),
        },
    )
    .unwrap();
    assert!(
        Answer::new(&input, Reply::Lease { grant })
            .unwrap()
            .reply(&input)
            .is_err()
    );
    let lease = Call::new(
        fixtures::target().address.hub_id,
        Command::Lease {
            request: fixtures::lease(),
        },
    )
    .unwrap();
    assert!(
        Answer::new(&lease, Reply::Input { duplicate: false })
            .unwrap()
            .reply(&lease)
            .is_err()
    );
}

#[test]
fn response_debug_exposes_shape_only_and_does_not_copy_payloads_or_error_messages() {
    let call = Call::from_request(commands::request(Command::Hello {}));
    let mut answer = Answer::new(
        &call,
        Reply::Error {
            code: PublicError::Unauthorized,
        },
    )
    .unwrap();
    answer.error.as_mut().unwrap().message = SECRET.into();
    answer.error.as_mut().unwrap().data = Some(json!(SECRET));
    assert!(!format!("{answer:?}").contains(SECRET));
    answer.result = Some(json!(SECRET));
    assert!(!format!("{answer:?}").contains(SECRET));
}
