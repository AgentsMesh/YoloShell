use crate::worker::tests::*;
const DISPATCH_UNKNOWN_SIGNAL: &str = "unknown";
const DISPATCH_FORCE_SIGNAL: &str = "KILL";

#[tokio::test]
async fn request_dispatch_covers_status_snapshot_resize_signal_and_termination() {
    let fixture = Fixture::with_contained_command(CommandBuilder::new(FIXTURE_COMMAND));
    let state = &fixture.state;
    assert!(
        dispatch_test(state, WORKER_STATUS_METHOD, &())
            .await
            .unwrap()
            .ok
    );
    let running_shutdown = dispatch_test(
        state,
        WORKER_SHUTDOWN_METHOD,
        &TerminalRef {
            terminal_id: fixture.terminal_id(),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(running_shutdown.code, "conflict");
    assert!(
        dispatch_test(
            state,
            method::GET_SNAPSHOT,
            &SnapshotRequest {
                terminal_id: fixture.terminal_id(),
                runtime_epoch: state.runtime_epoch(),
            },
        )
        .await
        .unwrap()
        .ok
    );

    let acquired = dispatch_test(
        state,
        method::ACQUIRE_WRITER,
        &fixture.lease_request("dispatch"),
    )
    .await
    .unwrap();
    let lease: WriterLeaseGrant = acquired.decode_payload().unwrap();
    let renewed = dispatch_test(
        state,
        method::RENEW_WRITER,
        &LeaseRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: state.runtime_epoch(),
            holder_id: "dispatch".into(),
            ttl_ms: Some(3_000),
            generation: Some(lease.generation),
            controller_instance_id: None,
            writer_capability: lease.writer_capability.clone(),
        },
    )
    .await
    .unwrap();
    assert!(renewed.ok);
    let input = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        input_id: Uuid::new_v4(),
        data: b"dispatch input\n".to_vec(),
    };
    assert!(
        dispatch_test(state, method::TERMINAL_INPUT, &input)
            .await
            .unwrap()
            .ok
    );
    let takeover = dispatch_test(
        state,
        method::TAKEOVER_WRITER,
        &LeaseRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: state.runtime_epoch(),
            holder_id: "takeover".into(),
            ttl_ms: None,
            generation: Some(lease.generation),
            controller_instance_id: None,
            writer_capability: WriterCapability::generate(),
        },
    )
    .await
    .unwrap();
    let takeover_lease: WriterLeaseGrant = takeover.decode_payload().unwrap();
    assert!(takeover_lease.generation > lease.generation);
    assert!(
        dispatch_test(state, method::TERMINAL_INPUT, &input)
            .await
            .is_err()
    );
    let invalid_resize = ResizeRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: state.runtime_epoch(),
        generation: takeover_lease.generation,
        writer_capability: takeover_lease.writer_capability.clone(),
        cols: 0,
        rows: 24,
    };
    assert_eq!(
        dispatch_test(state, method::RESIZE_TERMINAL, &invalid_resize)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::INVALID_REQUEST
    );
    dispatch_test(
        state,
        method::RESIZE_TERMINAL,
        &ResizeRequest {
            cols: 120,
            rows: 40,
            ..invalid_resize
        },
    )
    .await
    .unwrap();
    let invalid_signal = SignalRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: state.runtime_epoch(),
        generation: takeover_lease.generation,
        writer_capability: takeover_lease.writer_capability.clone(),
        signal: DISPATCH_UNKNOWN_SIGNAL.into(),
    };
    assert_eq!(
        dispatch_test(state, method::SIGNAL_SESSION, &invalid_signal)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::INVALID_REQUEST
    );
    let force_signal = SignalRequest {
        signal: DISPATCH_FORCE_SIGNAL.into(),
        ..invalid_signal.clone()
    };
    assert_eq!(
        dispatch_test(state, method::SIGNAL_SESSION, &force_signal)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::INVALID_REQUEST
    );
    assert_eq!(
        dispatch_test(
            state,
            method::GET_SNAPSHOT,
            &SnapshotRequest {
                terminal_id: Uuid::new_v4(),
                runtime_epoch: state.runtime_epoch(),
            },
        )
        .await
        .unwrap_err()
        .code,
        crate::protocol::error_code::IDENTITY_MISMATCH
    );
    dispatch_test(
        state,
        method::SIGNAL_SESSION,
        &SignalRequest {
            signal: "CONT".into(),
            ..invalid_signal
        },
    )
    .await
    .unwrap();
    dispatch_test(
        state,
        method::RELEASE_WRITER,
        &LeaseRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: state.runtime_epoch(),
            holder_id: "takeover".into(),
            ttl_ms: None,
            generation: Some(takeover_lease.generation),
            controller_instance_id: None,
            writer_capability: takeover_lease.writer_capability.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(*state.dimensions.lock().unwrap(), (120, 40));
    assert_eq!(
        dispatch_test(state, "no_such_method", &())
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::UNSUPPORTED_METHOD
    );

    assert_dispatch_termination(&fixture, state).await;
}
