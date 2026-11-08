use crate::host::tests::*;
use super::*;

include!("../../test_support/unit/dispatch_constants.rs");

fn malformed_dispatch_request(method_name: &str) -> RequestEnvelope {
    RequestEnvelope::new(
        format!("{MALFORMED_DISPATCH_ID_PREFIX}{method_name}"),
        method_name,
        yolo_capnp_value::Value::from(MALFORMED_DISPATCH_PAYLOAD),
    )
}

#[tokio::test]
async fn dispatch_rejects_every_strict_payload_without_losing_correlation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    for method_name in [
        method::GET_SESSION,
        method::LIST_SESSIONS,
        method::CREATE_SESSION,
        method::ARCHIVE_SESSION,
        method::TERMINATE_SESSION,
        method::DISCARD_SESSION,
        method::CLEANUP_TERMINATE_SESSION,
        method::CLEANUP_ARCHIVE_SESSION,
        method::CLEANUP_DISCARD_SESSION,
        method::FINALIZE_CREATION,
        method::RESUME_SESSION_EXACT,
    ] {
        let request = malformed_dispatch_request(method_name);
        let response = host.dispatch(&request).await;
        assert_eq!(response.request_id, request.request_id);
        assert_eq!(response.method, request.method);
        let error = response.error.unwrap();
        assert_eq!(error.code, crate::protocol::error_code::INVALID_REQUEST);
        assert!(!error.retryable);
    }
}

#[tokio::test]
async fn dispatch_rejects_malformed_hello_and_retired_methods() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let hello = host
        .dispatch(&malformed_dispatch_request(method::HELLO))
        .await;
    assert_eq!(
        hello.error.unwrap().code,
        crate::protocol::error_code::INVALID_REQUEST
    );
    for method_name in RETIRED_HOST_METHODS {
        let request = request(
            method_name,
            &ResumeRequest {
                selection: TerminalLifecycleSelection::unconditional(Uuid::new_v4()),
                recreate: false,
            },
        );
        let response = host.dispatch(&request).await;
        assert_eq!(response.request_id, request.request_id);
        assert_eq!(response.method, method_name);
        let error = response.error.unwrap();
        assert_eq!(error.code, crate::protocol::error_code::UNSUPPORTED_METHOD);
        assert!(!error.retryable);
        assert!(!yolo_host_change_feed::mutates(method_name));
    }
}

#[tokio::test]
async fn dispatch_enters_each_lifecycle_handler_before_returning_missing() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let creation_capability = Uuid::new_v4();
    let requests = [
        request(
            method::DISCARD_SESSION,
            &TerminalDiscardRequest {
                selection: TerminalLifecycleSelection::unconditional(terminal_id),
                force: false,
            },
        ),
        request(
            method::CLEANUP_TERMINATE_SESSION,
            &CreationCleanupTerminateRequest {
                terminal_id: terminal_id,
                creation_capability,
                force: false,
            },
        ),
        request(
            method::CLEANUP_ARCHIVE_SESSION,
            &CreationCleanupArchiveRequest {
                terminal_id: terminal_id,
                creation_capability,
            },
        ),
        request(
            method::CLEANUP_DISCARD_SESSION,
            &CreationCleanupDiscardRequest {
                terminal_id: terminal_id,
                creation_capability,
                force: false,
            },
        ),
        request(
            method::FINALIZE_CREATION,
            &FinalizeCreationRequest {
                creation_token: Uuid::new_v4(),
                terminal_id: terminal_id,
                creation_capability,
            },
        ),
    ];
    for request in requests {
        let response = host.dispatch(&request).await;
        assert!(!response.ok);
        assert!(!response.error.unwrap().retryable);
    }
}
