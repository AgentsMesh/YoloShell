use crate::worker::tests::*;
#[tokio::test]
async fn input_is_idempotent_and_empty_or_stale_input_is_rejected() {
    let fixture = Fixture::new();
    let state = &fixture.state;
    let controller = fixture.controller();
    let lease = state
        .acquire_lease(controller, &fixture.lease_request("writer"), false)
        .await
        .unwrap();
    let input_id = Uuid::new_v4();
    let request = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        input_id,
        data: b"hello\n".to_vec(),
    };
    assert!(
        !state
            .write_input(controller, &request)
            .await
            .unwrap()
            .duplicate
    );
    assert!(
        state
            .write_input(controller, &request)
            .await
            .unwrap()
            .duplicate
    );
    let evicted = Uuid::new_v4();
    {
        let mut delivered = state.delivered_inputs.lock().unwrap();
        delivered.record(
            &lease.writer_capability,
            evicted,
            b"evicted",
            CachedInputDelivery::Delivered,
            DELIVERED_INPUT_CACHE_LIMIT,
        );
        for _ in 1..DELIVERED_INPUT_CACHE_LIMIT {
            let input_id = Uuid::new_v4();
            delivered.record(
                &lease.writer_capability,
                input_id,
                b"retained",
                CachedInputDelivery::Delivered,
                DELIVERED_INPUT_CACHE_LIMIT,
            );
        }
    }
    let mut distinct = request.clone();
    distinct.input_id = Uuid::new_v4();
    assert!(
        !state
            .write_input(controller, &distinct)
            .await
            .unwrap()
            .duplicate
    );
    assert_eq!(
        state.delivered_inputs.lock().unwrap().len(),
        DELIVERED_INPUT_CACHE_LIMIT
    );
    assert_eq!(
        state
            .delivered_inputs
            .lock()
            .unwrap()
            .get(&lease.writer_capability, evicted, b"evicted"),
        CachedInputLookup::Missing
    );
    let mut empty = request.clone();
    empty.input_id = Uuid::new_v4();
    empty.data.clear();
    assert!(state.write_input(controller, &empty).await.is_err());
    empty.data = b"stale".to_vec();
    empty.generation += 99;
    assert!(state.write_input(controller, &empty).await.is_err());
}

#[tokio::test]
async fn pending_delivery_survives_other_holder_cache_eviction() {
    const ORIGINAL_HOLDER: &str = "pending-owner";
    const OTHER_HOLDER: &str = "cache-filler";
    const CACHE_PRESSURE_RECORDS: usize = DELIVERED_INPUT_CACHE_LIMIT + 1;
    const ORIGINAL_DATA: &[u8] = b"execute-once\n";
    const FILLER_DATA: &[u8] = b"filler";

    let fixture = Fixture::new();
    let controller = fixture.controller();
    let first_request = fixture.lease_request(ORIGINAL_HOLDER);
    let first = fixture
        .state
        .acquire_lease(controller, &first_request, false)
        .await
        .unwrap();
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(SharedInputWriter {
        bytes: Arc::new(Mutex::new(Vec::new())),
        writes: Arc::clone(&writes),
        fail: false,
    }));
    let original = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: first.generation,
        writer_capability: first.writer_capability.clone(),
        input_id: Uuid::new_v4(),
        data: ORIGINAL_DATA.to_vec(),
    };
    fixture
        .state
        .write_input(controller, &original)
        .await
        .unwrap();

    let mut other_request = fixture.lease_request(OTHER_HOLDER);
    other_request.generation = Some(first.generation);
    let other = fixture
        .state
        .acquire_lease(controller, &other_request, true)
        .await
        .unwrap();
    {
        let mut delivered = fixture.state.delivered_inputs.lock().unwrap();
        for _ in 0..CACHE_PRESSURE_RECORDS {
            delivered.record(
                &other.writer_capability,
                Uuid::new_v4(),
                FILLER_DATA,
                CachedInputDelivery::Delivered,
                DELIVERED_INPUT_CACHE_LIMIT,
            );
        }
    }

    let mut reacquire = first_request;
    reacquire.generation = Some(other.generation);
    let reacquired = fixture
        .state
        .acquire_lease(controller, &reacquire, true)
        .await
        .unwrap();
    let mut retry = original;
    retry.generation = reacquired.generation;
    assert!(
        fixture
            .state
            .write_input(controller, &retry)
            .await
            .unwrap()
            .duplicate
    );
    assert_eq!(writes.load(Ordering::Acquire), 1);
}
