use super::*;

#[tokio::test]
async fn native_interrupt_transmits_exact_writer_witness_and_request_identity() {
    let request = interrupt_request();
    let expected = result(&request, NativeControlOutcome::Delivered);
    let server = ReplyServer::start(method::INTERRUPT_NATIVE_AGENT, &expected).await;
    assert_eq!(
        server
            .client
            .interrupt_native_agent(&request)
            .await
            .unwrap(),
        expected
    );
    let sent: NativeInterruptRequest = server.finish().await.decode_payload().unwrap();
    assert_eq!(sent.terminal_id, request.terminal_id);
    assert_eq!(sent.runtime_epoch, request.runtime_epoch);
    assert_eq!(sent.request_id, request.request_id);
    assert_eq!(sent.command_digest, request.command_digest);
    assert_eq!(sent.generation, request.generation);
    assert_eq!(sent.writer_capability, request.writer_capability);
    assert_eq!(sent.witness, request.witness);
}

#[tokio::test]
async fn native_interrupt_rejects_each_payload_correlation_mismatch() {
    let request = interrupt_request();
    for mutate in RESULT_MUTATIONS {
        let mut reply = result(&request, NativeControlOutcome::Delivered);
        mutate(&mut reply);
        let server = ReplyServer::start(method::INTERRUPT_NATIVE_AGENT, &reply).await;
        assert_mismatch(server.client.interrupt_native_agent(&request).await);
        server.finish().await;
    }
}

#[tokio::test]
async fn native_control_remote_errors_remain_errors_without_fabricating_receipts() {
    let request = interrupt_request();
    let target = RuntimeRef::new(request.terminal_id, request.runtime_epoch);
    for method_name in [
        method::OBSERVE_NATIVE_AGENT,
        method::GET_NATIVE_CONTROL_RECEIPT,
        method::INTERRUPT_NATIVE_AGENT,
    ] {
        let server = ReplyServer::failure(
            method_name,
            crate::protocol::ErrorBody::conflict(REMOTE_DENIAL),
        )
        .await;
        let error = match method_name {
            method::OBSERVE_NATIVE_AGENT => server
                .client
                .native_agent_observation(target, request.witness.fields().binding_digest)
                .await
                .unwrap_err(),
            method::GET_NATIVE_CONTROL_RECEIPT => server
                .client
                .native_control_receipt(target, request.request_id, request.command_digest)
                .await
                .unwrap_err(),
            _ => server
                .client
                .interrupt_native_agent(&request)
                .await
                .unwrap_err(),
        };
        assert!(
            matches!(error, ClientError::Remote {code, message, retryable: false, ..}
            if code == crate::protocol::error_code::CONFLICT && message == REMOTE_DENIAL)
        );
        server.finish().await;
    }
}
