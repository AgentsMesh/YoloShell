use crate::worker::tests::*;
const STALE_RESIZE_COLUMNS: u16 = 120;
const STALE_RESIZE_ROWS: u16 = 40;

#[tokio::test]
async fn control_epoch_worker_rejects_stale_requests_before_side_effects() {
    let fixture = Fixture::new();
    let state = &fixture.state;
    let controller = fixture.controller();
    let mut stale_acquire = fixture.lease_request("stale-runtime");
    stale_acquire.runtime_epoch = Uuid::new_v4();
    let error = dispatch_test(state, method::ACQUIRE_WRITER, &stale_acquire)
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert!(state.get_writer_lease(controller).await.unwrap().is_none());

    let lease = state
        .acquire_lease(controller, &fixture.lease_request("current-runtime"), false)
        .await
        .unwrap();
    let stale_epoch = Uuid::new_v4();
    let stale_target = RuntimeRef::new(fixture.terminal_id(), stale_epoch);
    let error = dispatch_test(state, method::GET_WRITER_LEASE, &stale_target)
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert_eq!(
        state
            .get_writer_lease(controller)
            .await
            .unwrap()
            .unwrap()
            .generation,
        lease.generation
    );
    let snapshot_path = state.paths.snapshot(fixture.terminal_id());
    let snapshot_before = std::fs::read(&snapshot_path).ok();
    let stale_snapshot = SnapshotRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: stale_epoch,
    };
    let error = dispatch_test(state, method::GET_SNAPSHOT, &stale_snapshot)
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert_eq!(std::fs::read(snapshot_path).ok(), snapshot_before);
    let dimensions_before = *lock_recover(&state.dimensions);
    let stale_resize = ResizeRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: stale_epoch,
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        cols: STALE_RESIZE_COLUMNS,
        rows: STALE_RESIZE_ROWS,
    };
    let error = dispatch_test(state, method::RESIZE_TERMINAL, &stale_resize)
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert_eq!(*lock_recover(&state.dimensions), dimensions_before);
    let input_id = Uuid::new_v4();
    let data = b"stale-runtime-input".to_vec();
    let stale_input = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: stale_epoch,
        generation: lease.generation,
        writer_capability: lease.writer_capability,
        input_id,
        data: data.clone(),
    };
    let error = dispatch_test(state, method::TERMINAL_INPUT, &stale_input)
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert!(matches!(
        lock_recover(&state.delivered_inputs).get(
            &stale_input.writer_capability,
            input_id,
            &data
        ),
        CachedInputLookup::Missing
    ));
}
