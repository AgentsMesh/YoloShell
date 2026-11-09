use super::*;

#[test]
fn remote_error_keeps_retryability_and_details() {
    let client = Client::new("/tmp/yolo-client-test.sock");
    assert_eq!(
        client.socket_path(),
        Path::new("/tmp/yolo-client-test.sock")
    );
    let error = ClientError::from(ErrorBody {
        code: "conflict".into(),
        message: "held".into(),
        retryable: true,
        details: Some(serde_json::Value::String("writer-a".into())),
    });
    match error {
        ClientError::Remote {
            code,
            retryable,
            details,
            ..
        } => {
            assert_eq!(code, "conflict");
            assert!(retryable);
            assert_eq!(details, Some(serde_json::Value::String("writer-a".into())));
        }
        _ => panic!("unexpected error variant"),
    }
}
