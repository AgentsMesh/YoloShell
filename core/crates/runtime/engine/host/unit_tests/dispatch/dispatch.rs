use super::*;
use crate::host::tests::*;

const DISPATCH_MISSING_RUNTIME: &str = "missing-runtime";

#[tokio::test]
async fn host_dispatches_catalog_hello_and_validation_paths() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join(DISPATCH_MISSING_RUNTIME));
    let expected_socket = Paths::new(directory.path()).host_socket().unwrap();
    let host = Host::open(config).unwrap();
    assert_eq!(host.socket_path(), expected_socket);
    assert_eq!(host.controller_generation(), 1);

    let hello = host
        .dispatch(&request(
            method::HELLO,
            &HelloRequest {
                client_name: Some("test".into()),
                protocol_version: crate::model::PROTOCOL_VERSION,
                capabilities: vec!["binary".into()],
            },
        ))
        .await;
    assert!(hello.ok);
    const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;
    const INCOMPATIBLE_PROTOCOL_VERSION: u16 =
        crate::model::PROTOCOL_VERSION.saturating_add(INCOMPATIBLE_PROTOCOL_VERSION_OFFSET);
    let incompatible = host
        .dispatch(&request(
            method::HELLO,
            &HelloRequest {
                client_name: None,
                protocol_version: INCOMPATIBLE_PROTOCOL_VERSION,
                capabilities: Vec::new(),
            },
        ))
        .await;
    let incompatible = incompatible.error.unwrap();
    assert_eq!(
        incompatible.code,
        crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
    );
    assert!(
        incompatible
            .message
            .contains(&crate::model::PROTOCOL_VERSION.to_string())
    );
    assert!(host.dispatch(&request(method::HOST_STATUS, &())).await.ok);
    let hello_without_payload = host
        .dispatch(&RequestEnvelope::new(
            "hello-nil",
            method::HELLO,
            yolo_capnp_value::Value::Null,
        ))
        .await;
    assert_eq!(
        hello_without_payload.error.unwrap().code,
        crate::protocol::error_code::INVALID_REQUEST
    );

    let empty = host
        .dispatch(&request(
            method::LIST_SESSIONS,
            &ListSessionsRequest {
                include_archived: false,
                require_complete: false,
            },
        ))
        .await;
    assert!(empty.ok);
    let empty: ListSessionsResult = empty.decode_payload().unwrap();
    assert!(empty.complete);
    let missing = host
        .dispatch(&request(
            method::GET_SESSION,
            &TerminalRef {
                terminal_id: Uuid::new_v4(),
            },
        ))
        .await;
    assert_eq!(
        missing.error.unwrap().code,
        crate::protocol::error_code::NOT_FOUND
    );
    let missing_resume = host
        .dispatch(&request(
            method::RESUME_SESSION_EXACT,
            &ResumeRequest {
                selection: TerminalLifecycleSelection::unconditional(Uuid::new_v4()),
                recreate: false,
            },
        ))
        .await;
    assert_eq!(
        missing_resume.error.unwrap().code,
        crate::protocol::error_code::NOT_FOUND
    );
    let unsupported = host.dispatch(&request("future_method", &())).await;
    assert_eq!(
        unsupported.error.unwrap().code,
        crate::protocol::error_code::UNSUPPORTED_METHOD
    );

    for cwd in [
        PathBuf::from("relative"),
        PathBuf::from("~other"),
        directory.path().join("missing"),
    ] {
        let invalid = host
            .dispatch(&request(
                method::CREATE_SESSION,
                &CreateTerminal {
                    creation_token: Uuid::new_v4(),
                    name: None,
                    cwd,
                    cols: None,
                    rows: None,
                    initial_input: None,
                    context: None,
                },
            ))
            .await;
        assert_eq!(
            invalid.error.unwrap().code,
            crate::protocol::error_code::INVALID_DIRECTORY
        );
    }
    let nil_token = host
        .dispatch(&request(
            method::CREATE_SESSION,
            &CreateTerminal {
                creation_token: Uuid::nil(),
                name: None,
                cwd: directory.path().to_path_buf(),
                cols: None,
                rows: None,
                initial_input: None,
                context: None,
            },
        ))
        .await;
    assert_eq!(
        nil_token.error.unwrap().code,
        crate::protocol::error_code::INVALID_REQUEST
    );
    let legacy_payload = yolo_capnp_value::Value::Map(vec![
        (
            yolo_capnp_value::Value::from("cwd"),
            crate::protocol::to_value(&directory.path().to_path_buf()).unwrap(),
        ),
        (
            yolo_capnp_value::Value::from("provider"),
            yolo_capnp_value::Value::from("generic"),
        ),
    ]);
    let legacy = host
        .dispatch(&RequestEnvelope::new(
            "legacy-create",
            method::CREATE_SESSION,
            legacy_payload,
        ))
        .await;
    assert_eq!(
        legacy.error.unwrap().code,
        crate::protocol::error_code::INVALID_REQUEST
    );
    let worker_start_failure = host
        .create_session(CreateTerminal {
            creation_token: Uuid::new_v4(),
            name: None,
            cwd: directory.path().to_path_buf(),
            cols: None,
            rows: None,
            initial_input: None,
            context: None,
        })
        .await;
    assert_eq!(
        worker_start_failure.unwrap_err().code,
        crate::protocol::error_code::WORKER_START_FAILED
    );
}
