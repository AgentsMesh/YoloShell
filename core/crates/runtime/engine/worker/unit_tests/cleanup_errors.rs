#[tokio::test]
async fn deferred_cleanup_has_a_typed_non_retryable_protocol_error() {
    let fixture = Fixture::new();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    settlement
        .set(Err(
            pty_owner_handle::PtyOwnerCommandFailure::CleanupDeferred(
                termination_timing::CapabilitySettlementDeferred,
            ),
        ))
        .unwrap();
    let (commands, _receiver) = std::sync::mpsc::channel();
    let previous = lock_recover(&fixture.state.pty_owner).replace(
        pty_owner_handle::PtyOwnerHandle::for_test(commands, settlement),
    );
    assert!(previous.is_none());
    let request = RequestEnvelope::new(
        CLEANUP_DEFERRED_REQUEST_ID,
        method::TERMINATE_RUNTIME_EXACT,
        to_value(&FencedTerminateRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: fixture.state.runtime_epoch(),
            expected_status: None,
            force: true,
        })
        .unwrap(),
    );

    let response = dispatch_request(&request, &fixture.state, fixture.controller()).await;
    let error = response.error.unwrap();

    assert_eq!(error.code, crate::protocol::error_code::CLEANUP_DEFERRED);
    assert!(!error.retryable);
}
