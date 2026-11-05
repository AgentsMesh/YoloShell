use super::*;

#[tokio::test]
async fn every_native_method_dispatches_to_the_exact_runtime_and_preserves_payload_correlation() {
    for &method_name in NATIVE_METHODS {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        let session = Uuid::new_v4();
        host.paths.ensure_session(session).unwrap();
        let socket = directory.path().join(NATIVE_SOCKET);
        let listener = UnixListener::bind(&socket).unwrap();
        let runtime = host_runtime_record(&host, session, socket);
        atomic_write_json(&host.paths.runtime_record(session), &runtime).unwrap();
        let server_runtime = runtime.clone();
        let server = tokio::spawn(async move {
            tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut stream = UnixStream::from(stream);
                let mut stream = accept_worker_auth(stream, &server_runtime).await;
                let forwarded: RequestEnvelope = stream.request().await.unwrap();
                assert_eq!(forwarded.method, method_name);
                let mut response = ResponseEnvelope::empty(&forwarded);
                response.payload = forwarded.payload.clone();
                stream.respond(&response).await.unwrap();
                forwarded.payload
            })
            .await
            .unwrap()
        });
        let payload = opaque_native_payload(session, runtime.runtime_epoch);
        let request = request(method_name, &payload);
        let response = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, host.dispatch(&request))
            .await
            .unwrap();
        assert!(response.ok);
        assert_eq!(response.request_id, request.request_id);
        assert_eq!(response.method, request.method);
        assert_eq!(response.payload, payload);
        assert_eq!(server.await.unwrap(), payload);
    }
}

#[tokio::test]
async fn native_dispatch_rejects_replaced_runtime_before_connecting_to_its_worker() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let session = Uuid::new_v4();
    host.paths.ensure_session(session).unwrap();
    let socket = directory.path().join(NATIVE_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let runtime = host_runtime_record(&host, session, socket);
    atomic_write_json(&host.paths.runtime_record(session), &runtime).unwrap();
    let stale_epoch = Uuid::new_v4();
    for &method_name in NATIVE_METHODS {
        let request = request(method_name, &opaque_native_payload(session, stale_epoch));
        let response = host.dispatch(&request).await;
        assert_eq!(response.request_id, request.request_id);
        assert_eq!(response.method, request.method);
        assert_eq!(
            response.error.unwrap().code,
            crate::protocol::error_code::IDENTITY_MISMATCH
        );
    }
    assert!(
        tokio::time::timeout(NO_NATIVE_CONNECTION_TIMEOUT, listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn native_dispatch_rejects_malformed_scope_and_reports_absent_runtime() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    for &method_name in NATIVE_METHODS {
        let malformed = request(method_name, &());
        let response = host.dispatch(&malformed).await;
        assert_eq!(response.request_id, malformed.request_id);
        assert_eq!(response.method, malformed.method);
        assert_eq!(
            response.error.unwrap().code,
            crate::protocol::error_code::INVALID_REQUEST
        );
        let missing = request(
            method_name,
            &opaque_native_payload(Uuid::new_v4(), Uuid::new_v4()),
        );
        let response = host.dispatch(&missing).await;
        assert_eq!(response.request_id, missing.request_id);
        assert_eq!(response.method, missing.method);
        assert_eq!(
            response.error.unwrap().code,
            crate::protocol::error_code::WORKER_UNREACHABLE
        );
    }
}
