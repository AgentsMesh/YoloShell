use crate::worker::tests::*;
const POISON_EXIT_CODE: i32 = 23;
const DUPLICATE_EXIT_CODE: i32 = 24;
const FINALIZATION_NOTIFY_TIMEOUT: Duration =
    crate::worker::termination_timing::WORKER_TERMINATION_TIMEOUT;
pub(super) const EXIT_PERSISTENCE_TIMEOUT: Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const SHUTDOWN_GRACE_SLACK: Duration = Duration::from_secs(2);

struct PanicReader;

impl Read for PanicReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        panic!("injected reader panic");
    }
}

fn poison<T>(mutex: &Mutex<T>) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = mutex.lock().unwrap();
        panic!("injected lifecycle poison");
    }));
}

#[test]
fn failed_reader_attempts_remain_undrained_until_confirmed_eof() {
    let fixture = Fixture::new();
    let error = supervision::catch_thread(supervision::PTY_READER_TASK, || {
        run_reader_attempt(Box::new(FailingIo), Arc::clone(&fixture.state))
    });
    assert!(error.is_err());
    let progress = lock_recover(&fixture.state.reader_lifecycle.progress);
    assert!(!progress.active);
    assert!(!progress.eof.is_observed());
    drop(progress);
    assert!(drain_reader(&fixture.state, Duration::ZERO));

    let panic = supervision::catch_thread(supervision::PTY_READER_TASK, || {
        run_reader_attempt(Box::new(PanicReader), Arc::clone(&fixture.state))
    });
    assert!(panic.is_err());
    let progress = lock_recover(&fixture.state.reader_lifecycle.progress);
    assert!(!progress.active);
    assert!(!progress.eof.is_observed());
    drop(progress);
    assert!(drain_reader(&fixture.state, Duration::ZERO));
}

#[tokio::test]
async fn poisoned_lifecycle_still_publishes_one_exit_and_shutdown() {
    let fixture = Fixture::new();
    let terminal_id = fixture.terminal_id();
    let mut events = fixture.state.events.subscribe();
    fixture
        .state
        .begin_termination_for_test(ExitReason::UserTerminate);
    fixture.state.reader_lifecycle.mark_drained();
    poison(&fixture.state.writer);
    poison(&fixture.state.master);
    poison(&fixture.state.lease);
    poison(&fixture.state.termination_intent);
    poison(&fixture.state.journal);
    poison(&fixture.state.exit);
    poison(&fixture.state.record);
    poison(&fixture.state.dimensions);
    poison(&fixture.state.terminal);
    poison(&fixture.state.reader_lifecycle.progress);

    assert_eq!(
        fixture.state.status().status,
        TerminalRuntimeStatus::Finalizing
    );
    assert!(fixture.state.current_snapshot_internal().is_ok());
    let lease_error = fixture
        .state
        .get_writer_lease(fixture.controller())
        .await
        .unwrap_err();
    assert_eq!(lease_error.code, lease_error_code::SESSION_NOT_RUNNING);
    fixture.state.begin_termination_for_test(ExitReason::Lost);

    finalize_on_thread(
        Arc::clone(&fixture.state),
        Some(POISON_EXIT_CODE),
        None,
        Duration::ZERO,
    )
    .await;
    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    assert_eq!(exit.code, Some(POISON_EXIT_CODE));
    assert_eq!(exit.reason, ExitReason::Lost);
    let stored: ExitRecord = read_json(&fixture.state.paths.exit_record(terminal_id)).unwrap();
    assert_eq!(stored.code, Some(POISON_EXIT_CODE));
    assert!(matches!(events.recv().await, Ok(WorkerEvent::Exited(_))));

    finalize_on_thread(
        Arc::clone(&fixture.state),
        Some(DUPLICATE_EXIT_CODE),
        None,
        Duration::ZERO,
    )
    .await;
    assert_eq!(
        lock_recover(&fixture.state.exit).as_ref().unwrap().code,
        Some(POISON_EXIT_CODE)
    );
    assert!(events.try_recv().is_err());
    tokio::time::timeout(
        FINALIZATION_NOTIFY_TIMEOUT,
        fixture.state.shutdown.notified(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn poisoned_record_cannot_interrupt_exit_before_reader_drain() {
    let fixture = Fixture::new();
    let terminal_id = fixture.terminal_id();
    let mut events = fixture.state.events.subscribe();
    fixture
        .state
        .begin_termination_for_test(ExitReason::UserTerminate);
    fixture.state.reader_lifecycle.mark_drained();
    poison(&fixture.state.record);

    finalize_on_thread(Arc::clone(&fixture.state), None, None, Duration::ZERO).await;

    assert!(read_json::<ExitRecord>(&fixture.state.paths.exit_record(terminal_id)).is_ok());
    assert!(matches!(events.recv().await, Ok(WorkerEvent::Exited(_))));
    tokio::time::timeout(
        FINALIZATION_NOTIFY_TIMEOUT,
        fixture.state.shutdown.notified(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn exit_is_not_published_until_its_record_is_durable() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();
    let exit_path = fixture.state.paths.exit_record(fixture.terminal_id());
    fixture.state.control_test_hooks.block_exit_persist();
    let state = Arc::clone(&fixture.state);
    let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
    let finalizer = std::thread::spawn(move || {
        finalize_child_exit_scheduled(
            &state,
            Some(POISON_EXIT_CODE),
            None,
            FinalizationSchedule {
                drain_interval: Duration::ZERO,
                blocked_after: Duration::ZERO,
            },
        );
        let _ = finished_tx.send(());
    });

    wait_for_runtime_status(
        &fixture.state,
        TerminalRuntimeStatus::OutputBlocked,
        EXIT_PERSISTENCE_TIMEOUT,
    )
    .await;
    assert!(lock_recover(&fixture.state.exit).is_none());
    assert!(fixture.state.namespace.prove_ownership_loss().is_none());
    assert_eq!(fixture.state.termination_reason(), None);
    fixture.state.control_test_hooks.allow_exit_persist();
    tokio::time::timeout(EXIT_PERSISTENCE_TIMEOUT, finished_rx)
        .await
        .unwrap()
        .unwrap();
    finalizer.join().unwrap();
    assert_eq!(fixture.state.status().status, TerminalRuntimeStatus::Exited);
    assert!(read_json::<ExitRecord>(&exit_path).is_ok());
}


include!("finalization_natural_exit.rs");
