use super::*;

#[test]
fn only_native_interrupt_is_a_host_mutation() {
    assert!(yolo_host_change_feed::mutates(method::INTERRUPT_NATIVE_AGENT));
    assert!(!yolo_host_change_feed::mutates(method::OBSERVE_NATIVE_AGENT));
    assert!(!yolo_host_change_feed::mutates(method::GET_NATIVE_CONTROL_RECEIPT));
}

#[tokio::test]
async fn draining_host_blocks_native_effects_while_observation_and_receipt_reach_dispatch() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let clients = Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CLIENTS));
    let preparation = host.request_drain.prepare().await.unwrap();
    let (mut client, task) = spawn_admitted_host_client(Arc::clone(&host), &clients);
    let client = open_host_rpc(client).await;
    for (method_name, error_code) in [
        (
            method::INTERRUPT_NATIVE_AGENT,
            crate::protocol::error_code::CONFLICT,
        ),
        (
            method::OBSERVE_NATIVE_AGENT,
            crate::protocol::error_code::WORKER_UNREACHABLE,
        ),
        (
            method::GET_NATIVE_CONTROL_RECEIPT,
            crate::protocol::error_code::WORKER_UNREACHABLE,
        ),
    ] {
        let request = request(
            method_name,
            &opaque_native_payload(Uuid::new_v4(), Uuid::new_v4()),
        );
        let response = host_rpc(&client, &request).await;
        assert_eq!(response.request_id, request.request_id);
        assert_eq!(response.method, request.method);
        assert_eq!(response.error.unwrap().code, error_code);
    }
    drop(preparation);
    let request = request(
        method::INTERRUPT_NATIVE_AGENT,
        &opaque_native_payload(Uuid::new_v4(), Uuid::new_v4()),
    );
    let response = host_rpc(&client, &request).await;
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::WORKER_UNREACHABLE
    );
    drop(client);
    let _ = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, task)
        .await
        .unwrap();
}
