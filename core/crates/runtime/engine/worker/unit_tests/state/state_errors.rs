use crate::worker::tests::*;
struct InterruptThenData {
    step: u8,
}

impl Read for InterruptThenData {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.step += 1;
        match self.step {
            1 => Err(std::io::ErrorKind::Interrupted.into()),
            2 => {
                buffer[..4].copy_from_slice(b"data");
                Ok(4)
            }
            _ => Ok(0),
        }
    }
}

#[tokio::test]
async fn initial_input_and_reader_errors_are_explicit() {
    let fixture = Fixture::new();
    fixture.state.writer.lock().unwrap().take();
    assert!(
        write_initial_input(Arc::clone(&fixture.state), b"input".to_vec())
            .await
            .is_err()
    );

    let fixture = Fixture::new();
    fixture.state.child_waited.store(true, Ordering::Release);
    reader_loop(
        Box::new(InterruptThenData { step: 0 }),
        Arc::clone(&fixture.state),
    )
    .unwrap();
    assert_eq!(fixture.state.journal.lock().unwrap().last_sequence(), 1);

    let fixture = Fixture::new();
    assert!(reader_loop(Box::new(FailingIo), Arc::clone(&fixture.state)).is_err());
}

#[tokio::test]
async fn pending_output_checkpoint_and_writer_failures_are_bounded() {
    let fixture = Fixture::new();
    stage_reader_output(&fixture.state, b"pending").unwrap();
    assert!(stage_reader_output(&fixture.state, b"replacement").is_err());
    assert_eq!(
        commit_staged_reader_output(&fixture.state).unwrap(),
        PendingDelivery::Delivered
    );
    let fixture = Fixture::new();
    for _ in 1..PTY_SNAPSHOT_INTERVAL_CHUNKS {
        fixture
            .state
            .journal
            .lock()
            .unwrap()
            .append(b"prior")
            .unwrap();
    }
    commit_reader_output(&fixture.state, b"checkpoint").unwrap();
    assert!(fixture.state.paths.snapshot(fixture.terminal_id()).exists());

    let lease = fixture
        .state
        .acquire_lease(
            fixture.controller(),
            &fixture.lease_request("writer"),
            false,
        )
        .await
        .unwrap();
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(FailingIo));
    let error = fixture
        .state
        .write_input(
            fixture.controller(),
            &InputRequest {
                terminal_id: fixture.terminal_id(),
                runtime_epoch: fixture.state.runtime_epoch(),
                generation: lease.generation,
                writer_capability: lease.writer_capability.clone(),
                input_id: Uuid::new_v4(),
                data: b"input".to_vec(),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert!(error.details.is_some());
}

#[tokio::test]
async fn delivered_input_cache_evicts_the_oldest_identifier() {
    let fixture = Fixture::new();
    let lease = fixture
        .state
        .acquire_lease(fixture.controller(), &fixture.lease_request("cache"), false)
        .await
        .unwrap();
    let oldest = Uuid::new_v4();
    {
        let mut delivered = fixture.state.delivered_inputs.lock().unwrap();
        delivered.record(
            &lease.writer_capability,
            oldest,
            b"oldest",
            CachedInputDelivery::Delivered,
            DELIVERED_INPUT_CACHE_LIMIT,
        );
        for _ in 1..DELIVERED_INPUT_CACHE_LIMIT {
            let id = Uuid::new_v4();
            delivered.record(
                &lease.writer_capability,
                id,
                b"newer",
                CachedInputDelivery::Delivered,
                DELIVERED_INPUT_CACHE_LIMIT,
            );
        }
    }
    fixture
        .state
        .write_input(
            fixture.controller(),
            &InputRequest {
                terminal_id: fixture.terminal_id(),
                runtime_epoch: fixture.state.runtime_epoch(),
                generation: lease.generation,
                writer_capability: lease.writer_capability.clone(),
                input_id: Uuid::new_v4(),
                data: b"cache\n".to_vec(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        fixture
            .state
            .delivered_inputs
            .lock()
            .unwrap()
            .get(&lease.writer_capability, oldest, b"oldest"),
        CachedInputLookup::Missing
    );
}
