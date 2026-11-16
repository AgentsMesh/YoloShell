use super::fixture::Fixture;
use crate::{constants::*, signature, wire::*};

fn request() -> Request {
    Request {
        version: VERSION,
        child: yolo_process::ProcessIdentity::capture(std::process::id())
            .unwrap()
            .into(),
        nonce: uuid::Uuid::new_v4(),
        sequence: u64::default(),
        action: Action::Hello {},
    }
}

#[test]
fn owner_signature_binds_challenge_child_scope_and_credential_digest() {
    let fixture = Fixture::new();
    let request = request();
    let statement = Statement {
        scope: fixture.scope,
        request: request.clone(),
        credentials_sha256: yolo_crypto::sha256_hex(&[]),
    };
    let mut signed = signature::sign(&fixture.owner, statement).unwrap();
    signature::verify(&signed, &request).unwrap();
    let mut replay = request.clone();
    replay.nonce = uuid::Uuid::new_v4();
    assert!(signature::verify(&signed, &replay).is_err());
    replay = request.clone();
    replay.child.start_time += COUNTER_STEP;
    assert!(signature::verify(&signed, &replay).is_err());
    signed.statement.credentials_sha256 = yolo_crypto::sha256_hex(b"another envelope");
    assert!(signature::verify(&signed, &request).is_err());
}

#[test]
fn wire_rejects_unknown_fields_and_invalid_challenges() {
    let request = request();
    request.validate().unwrap();
    let mut value = serde_json::to_value(&request).unwrap();
    value["unexpected"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<Request>(value).is_err());
    let mut request = request;
    request.nonce = uuid::Uuid::nil();
    assert!(request.validate().is_err());
}
