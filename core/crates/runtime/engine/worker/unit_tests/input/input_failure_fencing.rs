use crate::worker::tests::*;
const REPLACEMENT_CONTROLLER_GENERATION: u64 = 2;

#[tokio::test]
async fn partial_input_is_unknown_and_never_replayed_or_crosses_fencing() {
    let fixture = Fixture::new();
    let old_controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(old_controller, &fixture.lease_request("partial"), false)
        .await
        .unwrap();
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(IncompleteInputWriter {
        writes: Arc::clone(&writes),
    }));
    let request = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        input_id: Uuid::new_v4(),
        data: b"partial".to_vec(),
    };
    for _ in 0..2 {
        let error = fixture
            .state
            .write_input(old_controller, &request)
            .await
            .unwrap_err();
        assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    }
    assert_eq!(writes.load(Ordering::Acquire), 1);
    fixture
        .state
        .adopt_controller(REPLACEMENT_CONTROLLER_GENERATION)
        .await
        .unwrap();
    assert!(
        fixture
            .state
            .write_input(old_controller, &request)
            .await
            .is_err()
    );
    assert_eq!(writes.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn missing_pty_descriptor_fails_before_calling_the_writer() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(controller, &fixture.lease_request("missing-fd"), false)
        .await
        .unwrap();
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(SharedInputWriter {
        bytes: Arc::new(Mutex::new(Vec::new())),
        writes: Arc::clone(&writes),
        fail: false,
    }));
    let _master = install_blocking_master(&fixture.state);
    let error = fixture
        .state
        .write_input(
            controller,
            &InputRequest {
                terminal_id: fixture.terminal_id(),
                runtime_epoch: fixture.state.runtime_epoch(),
                generation: lease.generation,
                writer_capability: lease.writer_capability.clone(),
                input_id: Uuid::new_v4(),
                data: b"must-not-write".to_vec(),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert_eq!(writes.load(Ordering::Acquire), 0);
}

#[tokio::test]
async fn one_input_identifier_cannot_name_different_data() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(controller, &fixture.lease_request("payload-binding"), false)
        .await
        .unwrap();
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(SharedInputWriter {
        bytes: Arc::new(Mutex::new(Vec::new())),
        writes: Arc::clone(&writes),
        fail: false,
    }));
    let mut request = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        input_id: Uuid::new_v4(),
        data: b"first".to_vec(),
    };
    fixture
        .state
        .write_input(controller, &request)
        .await
        .unwrap();
    request.data = b"second".to_vec();
    let error = fixture
        .state
        .write_input(controller, &request)
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(writes.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn public_generation_cannot_authorize_terminal_mutations() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let grant = fixture
        .state
        .acquire_lease(controller, &fixture.lease_request("authorized"), false)
        .await
        .unwrap();
    let public = fixture
        .state
        .get_writer_lease(controller)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(public.generation, grant.generation);
    let wrong = WriterCapability::generate();
    let input = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: public.generation,
        writer_capability: wrong.clone(),
        input_id: Uuid::new_v4(),
        data: b"unauthorized".to_vec(),
    };
    let resize = ResizeRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: public.generation,
        writer_capability: wrong.clone(),
        cols: 100,
        rows: 40,
    };
    let signal = SignalRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: public.generation,
        writer_capability: wrong,
        signal: "CONT".into(),
    };
    assert_eq!(
        fixture
            .state
            .write_input(controller, &input)
            .await
            .unwrap_err()
            .code,
        lease_error_code::NOT_HOLDER
    );
    assert_eq!(
        fixture
            .state
            .resize_terminal(controller, &resize)
            .await
            .unwrap_err()
            .code,
        lease_error_code::NOT_HOLDER
    );
    assert_eq!(
        dispatch_test(&fixture.state, method::SIGNAL_SESSION, &signal)
            .await
            .unwrap_err()
            .code,
        lease_error_code::NOT_HOLDER
    );
}
