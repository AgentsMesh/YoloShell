use crate::worker::tests::*;
struct ClosedPty;

struct SignalingClosedPty {
    observed: Option<mpsc::SyncSender<()>>,
}

impl Read for ClosedPty {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(libc::EIO))
    }
}

impl Read for SignalingClosedPty {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        if let Some(observed) = self.observed.take() {
            observed.send(()).unwrap();
        }
        Err(std::io::Error::from_raw_os_error(libc::EIO))
    }
}

#[tokio::test]
async fn initial_input_distinguishes_write_and_flush_failures() {
    let fixture = Fixture::new();
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(FailingIo));
    assert!(
        write_initial_input(Arc::clone(&fixture.state), b"write".to_vec())
            .await
            .is_err()
    );

    let fixture = Fixture::new();
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(FlushFails));
    assert!(
        write_initial_input(Arc::clone(&fixture.state), b"flush".to_vec())
            .await
            .is_err()
    );
}

#[test]
fn reader_requires_child_exit_confirmation() {
    let fixture = Fixture::new();
    let reader = std::io::Cursor::new(Vec::<u8>::new());
    assert_eq!(
        reader_loop(Box::new(reader), Arc::clone(&fixture.state)).unwrap(),
        ReaderLoopOutcome::UnexpectedEof {
            output_observed: false,
        }
    );
}

#[test]
#[cfg(target_os = "macos")]
fn confirmed_pty_eio_marks_the_reader_drained() {
    let fixture = Fixture::new();
    fixture.state.child_waited.store(true, Ordering::Release);
    fixture
        .state
        .reader_lifecycle
        .mark_producers_gone_for_test();

    assert_eq!(
        reader_loop(Box::new(ClosedPty), Arc::clone(&fixture.state)).unwrap(),
        ReaderLoopOutcome::ChildExited
    );
    let progress = lock_recover(&fixture.state.reader_lifecycle.progress);
    assert!(progress.eof.is_observed());
    assert!(!progress.read_stopped);
    assert!(!progress.active);
    assert!(progress.pending.is_empty());
}

#[test]
#[cfg(target_os = "macos")]
fn unconfirmed_pty_eio_remains_unexpected_and_undrained() {
    let fixture = Fixture::new();

    assert_eq!(
        reader_loop(Box::new(ClosedPty), Arc::clone(&fixture.state)).unwrap(),
        ReaderLoopOutcome::UnexpectedEof {
            output_observed: false,
        }
    );
    let progress = lock_recover(&fixture.state.reader_lifecycle.progress);
    assert!(!progress.eof.is_observed());
    assert!(!progress.read_stopped);
    assert!(!progress.active);
    assert!(progress.pending.is_empty());
}

#[test]
#[cfg(target_os = "macos")]
fn pty_eio_accepts_child_exit_during_confirmation_window() {
    let fixture = Fixture::new();
    fixture
        .state
        .reader_lifecycle
        .mark_producers_gone_for_test();
    let state = Arc::clone(&fixture.state);
    let (observed_tx, observed_rx) = mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        reader_loop(
            Box::new(SignalingClosedPty {
                observed: Some(observed_tx),
            }),
            state,
        )
    });
    observed_rx
        .recv_timeout(PTY_READER_EXIT_CONFIRM_TIMEOUT)
        .unwrap();
    fixture.state.child_waited.store(true, Ordering::Release);

    assert_eq!(
        reader.join().unwrap().unwrap(),
        ReaderLoopOutcome::ChildExited
    );
    assert!(
        lock_recover(&fixture.state.reader_lifecycle.progress)
            .eof
            .is_observed()
    );
}

#[test]
fn checkpoint_and_exit_persistence_failures_are_contained() {
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
    let snapshot = fixture.state.paths.snapshot(fixture.terminal_id());
    std::fs::create_dir(&snapshot).unwrap();
    commit_reader_output(&fixture.state, b"checkpoint").unwrap();

    fixture.state.reader_lifecycle.mark_drained();
    finalize_child_exit(&fixture.state, Some(7), None, Duration::ZERO);
    assert_eq!(
        fixture.state.exit.lock().unwrap().as_ref().unwrap().code,
        Some(7)
    );
}

#[test]
fn output_compacts_a_full_journal_before_appending() {
    let fixture = Fixture::new();
    let mut journal = fixture.state.journal.lock().unwrap();
    let payload = vec![0_u8; journal.remaining_payload_capacity()];
    journal.append(&payload).unwrap();
    drop(journal);
    let chunk = fixture.state.commit_output(b"after-compaction").unwrap();
    assert_eq!(chunk.sequence, 2);
    assert_eq!(
        fixture
            .state
            .journal
            .lock()
            .unwrap()
            .retained_from_sequence(),
        2
    );
}
