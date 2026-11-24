use super::*;

#[tokio::test]
async fn native_receipt_lookup_preserves_all_request_fields_and_each_outcome() {
    let request = interrupt_request();
    let target = RuntimeRef::new(request.terminal_id, request.runtime_epoch);
    for outcome in [
        NativeControlOutcome::Delivered,
        NativeControlOutcome::NotDelivered,
        NativeControlOutcome::Unknown,
    ] {
        let server = ReplyServer::start(
            method::GET_NATIVE_CONTROL_RECEIPT,
            &Some(result(&request, outcome)),
        )
        .await;
        assert_eq!(
            server
                .client
                .native_control_receipt(target, request.request_id, request.command_digest)
                .await
                .unwrap(),
            Some(outcome)
        );
        let sent: NativeControlLookupRequest = server.finish().await.decode_payload().unwrap();
        assert_eq!(sent.terminal_id, request.terminal_id);
        assert_eq!(sent.runtime_epoch, request.runtime_epoch);
        assert_eq!(sent.request_id, request.request_id);
        assert_eq!(sent.command_digest, request.command_digest);
    }
}

#[tokio::test]
async fn native_receipt_rejects_each_payload_correlation_mismatch() {
    let request = interrupt_request();
    let target = RuntimeRef::new(request.terminal_id, request.runtime_epoch);
    for mutate in RESULT_MUTATIONS {
        let mut reply = result(&request, NativeControlOutcome::Delivered);
        mutate(&mut reply);
        let server = ReplyServer::start(method::GET_NATIVE_CONTROL_RECEIPT, &Some(reply)).await;
        assert_mismatch(
            server
                .client
                .native_control_receipt(target, request.request_id, request.command_digest)
                .await,
        );
        server.finish().await;
    }
}

#[tokio::test]
async fn native_receipt_absence_stays_absent() {
    let request = interrupt_request();
    let server = ReplyServer::start(
        method::GET_NATIVE_CONTROL_RECEIPT,
        &Option::<NativeControlResult>::None,
    )
    .await;
    let target = RuntimeRef::new(request.terminal_id, request.runtime_epoch);
    assert!(
        server
            .client
            .native_control_receipt(target, request.request_id, request.command_digest)
            .await
            .unwrap()
            .is_none()
    );
    server.finish().await;
}
