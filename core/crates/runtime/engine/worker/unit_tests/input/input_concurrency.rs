use crate::worker::tests::*;
pub(super) struct SharedInputWriter {
    pub(super) bytes: Arc<Mutex<Vec<u8>>>,
    pub(super) writes: Arc<std::sync::atomic::AtomicUsize>,
    pub(super) fail: bool,
}

pub(super) struct IncompleteInputWriter {
    pub(super) writes: Arc<std::sync::atomic::AtomicUsize>,
}

impl Write for IncompleteInputWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.writes.fetch_add(1, Ordering::AcqRel);
        Ok(data.len().saturating_sub(1))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Write for SharedInputWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.writes.fetch_add(1, Ordering::AcqRel);
        if self.fail {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.bytes.lock().unwrap().extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn concurrent_duplicate_input_is_written_exactly_once() {
    let fixture = Fixture::new();
    let lease = fixture
        .state
        .acquire_lease(
            fixture.controller(),
            &fixture.lease_request("concurrent"),
            false,
        )
        .await
        .unwrap();
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(SharedInputWriter {
        bytes: Arc::clone(&bytes),
        writes: Arc::clone(&writes),
        fail: false,
    }));
    let input_id = Uuid::new_v4();
    let generation = lease.generation;
    let writer_capability = lease.writer_capability.clone();
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let controller = fixture.controller();
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let state = Arc::clone(&fixture.state);
        let barrier = Arc::clone(&barrier);
        let writer_capability = writer_capability.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            state
                .write_input(
                    controller,
                    &InputRequest {
                        terminal_id: state.terminal_id(),
                        runtime_epoch: state.runtime_epoch(),
                        generation,
                        writer_capability,
                        input_id,
                        data: b"single-write".to_vec(),
                    },
                )
                .await
        }));
    }
    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await.unwrap().unwrap());
    }
    assert_eq!(results.iter().filter(|result| result.duplicate).count(), 1);
    assert_eq!(writes.load(Ordering::Acquire), 1);
    assert_eq!(*bytes.lock().unwrap(), b"single-write");
}

#[tokio::test]
async fn unknown_input_is_never_retried_with_the_same_identifier() {
    let fixture = Fixture::new();
    let lease = fixture
        .state
        .acquire_lease(
            fixture.controller(),
            &fixture.lease_request("unknown"),
            false,
        )
        .await
        .unwrap();
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(SharedInputWriter {
        bytes: Arc::new(Mutex::new(Vec::new())),
        writes: Arc::clone(&writes),
        fail: true,
    }));
    let request = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        input_id: Uuid::new_v4(),
        data: b"unknown".to_vec(),
    };
    let first = fixture
        .state
        .write_input(fixture.controller(), &request)
        .await
        .unwrap_err();
    let second = fixture
        .state
        .write_input(fixture.controller(), &request)
        .await
        .unwrap_err();
    assert_eq!(first.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert_eq!(second.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert_eq!(writes.load(Ordering::Acquire), 1);
}
