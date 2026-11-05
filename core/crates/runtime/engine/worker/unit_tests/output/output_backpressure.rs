use super::*;

const RETRY_OBSERVATION: Duration = Duration::from_millis(350);
const STATUS_TIMEOUT: Duration = Duration::from_secs(2);
const DRAIN_INTERVAL: Duration = Duration::from_millis(20);
const OBSERVATION_POLL: Duration = Duration::from_millis(2);
const OUTPUT_CHUNKS: usize = PTY_OUTPUT_BATCH_CHUNK_LIMIT + 1;
const EXPECTED_READS: usize = OUTPUT_CHUNKS + 1;

fn chunk_payload(index: usize) -> Vec<u8> {
    format!("pending-output-{index}\r\n").into_bytes()
}

struct CountingBatchReader {
    reads: Arc<AtomicUsize>,
}

impl Read for CountingBatchReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let call = self.reads.fetch_add(1, Ordering::AcqRel);
        if call == OUTPUT_CHUNKS {
            return Ok(0);
        }
        let output = chunk_payload(call);
        buffer[..output.len()].copy_from_slice(&output);
        Ok(output.len())
    }
}

fn wait_for_failed_commit(state: &WorkerState) {
    let deadline = Instant::now() + STATUS_TIMEOUT;
    while state
        .reader_lifecycle
        .lock_progress()
        .pending_commit_retries
        .ordinary_failures
        == 0
    {
        assert!(
            Instant::now() < deadline,
            "pending batch did not attempt delivery"
        );
        std::thread::sleep(OBSERVATION_POLL);
    }
}

#[tokio::test]
async fn persistent_append_failure_reports_output_blocked_until_recovery() {
    let fixture = Fixture::with_event_capacity(WORKER_EVENT_CHANNEL_CAPACITY);
    let mut events = fixture.state.events.subscribe();
    fixture
        .state
        .journal
        .lock()
        .unwrap()
        .inject_append_failures(crate::journal::AppendStage::BeforeWrite, usize::MAX);
    let reads = Arc::new(AtomicUsize::default());
    let state = Arc::clone(&fixture.state);
    let reader_reads = Arc::clone(&reads);
    let reader = std::thread::spawn(move || {
        reader_loop(
            Box::new(CountingBatchReader {
                reads: reader_reads,
            }),
            state,
        )
    });

    wait_for_failed_commit(&fixture.state);
    std::thread::sleep(RETRY_OBSERVATION);
    assert_eq!(reads.load(Ordering::Acquire), PTY_OUTPUT_BATCH_CHUNK_LIMIT);
    assert_eq!(
        fixture.state.reader_lifecycle.pending_count(),
        PTY_OUTPUT_BATCH_CHUNK_LIMIT
    );
    assert_eq!(fixture.state.journal.lock().unwrap().last_sequence(), 0);
    assert!(events.try_recv().is_err());
    let finalizer_state = Arc::clone(&fixture.state);
    let finalizer = std::thread::spawn(move || {
        finalize_child_exit_scheduled(
            &finalizer_state,
            Some(0),
            None,
            FinalizationSchedule {
                drain_interval: DRAIN_INTERVAL,
                blocked_after: Duration::ZERO,
            },
        );
    });
    wait_for_runtime_status(
        &fixture.state,
        TerminalRuntimeStatus::OutputBlocked,
        STATUS_TIMEOUT,
    )
    .await;
    assert!(fixture.state.exit.lock().unwrap().is_none());
    assert!(fixture.state.master.lock().unwrap().is_some());
    assert!(!fixture.state.reader_lifecycle.is_cancelled());
    assert!(fixture.state.namespace.prove_ownership_loss().is_none());
    assert_eq!(fixture.state.termination_reason(), None);
    assert_eq!(reads.load(Ordering::Acquire), PTY_OUTPUT_BATCH_CHUNK_LIMIT);
    fixture
        .state
        .journal
        .lock()
        .unwrap()
        .clear_append_failures();
    assert_eq!(
        reader.join().unwrap().unwrap(),
        ReaderLoopOutcome::UnexpectedEof {
            output_observed: true
        }
    );
    assert!(fixture.state.exit.lock().unwrap().is_none());
    assert!(
        !fixture
            .state
            .reader_lifecycle
            .lock_progress()
            .eof
            .is_observed()
    );
    fixture
        .state
        .reader_lifecycle
        .mark_producers_gone_for_test();
    assert_eq!(
        reader_loop(Box::new(io::empty()), Arc::clone(&fixture.state)).unwrap(),
        ReaderLoopOutcome::ChildExited
    );
    finalizer.join().unwrap();
    assert_eq!(fixture.state.status().status, TerminalRuntimeStatus::Exited);
    assert_eq!(reads.load(Ordering::Acquire), EXPECTED_READS);
    let progress = fixture.state.reader_lifecycle.lock_progress();
    assert!(progress.eof.is_observed());
    assert!(!progress.read_stopped);
    assert!(progress.pending.is_empty());
    drop(progress);
    let chunks = fixture
        .state
        .journal
        .lock()
        .unwrap()
        .replay_after(0, usize::MAX)
        .unwrap();
    assert_eq!(chunks.len(), OUTPUT_CHUNKS);
    for (index, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.sequence, index as u64 + 1);
        assert_eq!(chunk.data, chunk_payload(index));
    }
    let mut delivered = Vec::new();
    loop {
        match events.try_recv() {
            Ok(WorkerEvent::Output(chunk)) => delivered.push(chunk),
            Ok(_) => {}
            Err(broadcast::error::TryRecvError::Empty) => break,
            Err(error) => panic!("output event stream interrupted: {error}"),
        }
    }
    assert_eq!(delivered.len(), chunks.len());
    for (event, durable) in delivered.iter().zip(&chunks) {
        assert_eq!(event.runtime_epoch, durable.runtime_epoch);
        assert_eq!(event.sequence, durable.sequence);
        assert_eq!(event.data, durable.data);
    }
    let exit = fixture.state.exit.lock().unwrap();
    let exit = exit.as_ref().unwrap();
    assert_eq!(exit.last_output_sequence, OUTPUT_CHUNKS as u64);
    assert_eq!(exit.reason, ExitReason::Natural);
}
