const READER_EXIT_RECOVERY_TIMEOUT: Duration = Duration::from_secs(2);
const READER_EXIT_INPUT: &[u8] = b"reader-exit-tail\n";
const READER_EXIT_MARKER: &[u8] = b"reader-exit-tail";
const READER_BACKOFF_NOT_ENTERED_DETAIL: &str = "PTY reader did not enter restart backoff";
const READER_BACKOFF_CHANNEL_CAPACITY: usize = 1;
const READER_RESTART_WAKE_TIMEOUT: Duration = Duration::from_secs(1);
const READER_EXIT_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(10);
#[derive(Clone, Copy)]
enum RestartWake {
    ChildExit,
    Cancellation,
}
#[tokio::test]
async fn child_exit_during_reader_backoff_reopens_and_drains_the_master() {
    let fixture = Fixture::new();
    let child_pid = fixture.child_pid();
    {
        let mut writer = fixture.state.writer.lock().unwrap();
        let writer = writer.as_mut().unwrap();
        writer.write_all(READER_EXIT_INPUT).unwrap();
        writer.flush().unwrap();
    }
    let (backoff_tx, backoff_rx) = mpsc::sync_channel(READER_BACKOFF_CHANNEL_CAPACITY);
    fixture
        .state
        .reader_lifecycle
        .install_restart_wait_probe(backoff_tx);
    spawn_reader(Box::new(FailingIo), Arc::clone(&fixture.state)).unwrap();
    backoff_rx
        .recv_timeout(READER_EXIT_RECOVERY_TIMEOUT)
        .expect(READER_BACKOFF_NOT_ENTERED_DETAIL);
    fixture.start_child_waiter();
    let record = lock_recover(&fixture.state.record);
    process::ProcessIdentity {
        pid: child_pid,
        start_time: record.child_start_time,
    }
    .signal(libc::SIGTERM)
    .unwrap();
    drop(record);
    tokio::time::timeout(READER_EXIT_RECOVERY_TIMEOUT, async {
        while !fixture.state.child_waited.load(Ordering::Acquire) {
            tokio::time::sleep(READER_EXIT_WAIT_POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap();
    fixture
        .state
        .reader_lifecycle
        .mark_producers_gone_for_test();
    let exit = wait_for_exit(&fixture.state).await;
    assert_eq!(exit.reason, ExitReason::Natural);
    assert!(exit.last_output_sequence > 0);
    let chunks = fixture
        .state
        .journal
        .lock()
        .unwrap()
        .replay_after(0, usize::MAX)
        .unwrap();
    let output: Vec<u8> = chunks.into_iter().flat_map(|chunk| chunk.data).collect();
    assert!(
        output
            .windows(READER_EXIT_MARKER.len())
            .any(|window| window == READER_EXIT_MARKER)
    );
    assert!(fixture.state.master.lock().unwrap().is_none());
    wait_for_runtime_status(
        &fixture.state,
        TerminalRuntimeStatus::Exited,
        READER_EXIT_RECOVERY_TIMEOUT,
    )
    .await;
}

#[test]
fn maximum_reader_backoff_is_interruptible() {
    for wake in [RestartWake::ChildExit, RestartWake::Cancellation] {
        let lifecycle = Arc::new(ReaderLifecycle::default());
        let confirmed = Arc::new(AtomicBool::new(false));
        let (backoff_tx, backoff_rx) = mpsc::sync_channel(READER_BACKOFF_CHANNEL_CAPACITY);
        let (complete_tx, complete_rx) = mpsc::sync_channel(READER_BACKOFF_CHANNEL_CAPACITY);
        lifecycle.install_restart_wait_probe(backoff_tx);
        let waiter_lifecycle = Arc::clone(&lifecycle);
        let waiter_confirmed = Arc::clone(&confirmed);
        let waiter = std::thread::spawn(move || {
            waiter_lifecycle.wait_for_restart(
                SUPERVISION_RESTART_MAX_DELAY,
                waiter_confirmed.as_ref(),
                ReaderRestartPhase::AwaitingChildExit,
                ReaderRestartKind::Read,
            );
            complete_tx.send(()).unwrap();
        });
        backoff_rx
            .recv_timeout(READER_EXIT_RECOVERY_TIMEOUT)
            .expect(READER_BACKOFF_NOT_ENTERED_DETAIL);
        match wake {
            RestartWake::ChildExit => {
                confirmed.store(true, Ordering::Release);
                lifecycle.notify_child_exit_confirmation();
            }
            RestartWake::Cancellation => lifecycle.cancel(),
        }
        complete_rx
            .recv_timeout(READER_RESTART_WAKE_TIMEOUT)
            .unwrap();
        waiter.join().unwrap();
    }
}

#[test]
fn child_exit_before_restart_wait_is_observed_without_sleeping() {
    let lifecycle = ReaderLifecycle::default();
    let confirmed = AtomicBool::new(true);
    let (backoff_tx, backoff_rx) = mpsc::sync_channel(READER_BACKOFF_CHANNEL_CAPACITY);
    lifecycle.install_restart_wait_probe(backoff_tx);

    lifecycle.wait_for_restart(
        SUPERVISION_RESTART_MAX_DELAY,
        &confirmed,
        ReaderRestartPhase::AwaitingChildExit,
        ReaderRestartKind::Read,
    );

    assert!(matches!(
        backoff_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
}

#[test]
fn confirmed_exit_does_not_bypass_final_drain_backoff() {
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let confirmed = Arc::new(AtomicBool::new(true));
    let (backoff_tx, backoff_rx) = mpsc::sync_channel(READER_BACKOFF_CHANNEL_CAPACITY);
    let (complete_tx, complete_rx) = mpsc::sync_channel(READER_BACKOFF_CHANNEL_CAPACITY);
    lifecycle.install_restart_wait_probe(backoff_tx);
    let waiter_lifecycle = Arc::clone(&lifecycle);
    let waiter_confirmed = Arc::clone(&confirmed);
    let waiter = std::thread::spawn(move || {
        waiter_lifecycle.wait_for_restart(
            SUPERVISION_RESTART_MAX_DELAY,
            waiter_confirmed.as_ref(),
            ReaderRestartPhase::FinalDrain,
            ReaderRestartKind::Read,
        );
        complete_tx.send(()).unwrap();
    });

    backoff_rx
        .recv_timeout(READER_EXIT_RECOVERY_TIMEOUT)
        .expect(READER_BACKOFF_NOT_ENTERED_DETAIL);
    assert!(matches!(
        complete_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    lifecycle.cancel();
    complete_rx
        .recv_timeout(READER_RESTART_WAKE_TIMEOUT)
        .unwrap();
    waiter.join().unwrap();
}

#[test]
fn cancelled_pending_restart_keeps_backoff_until_child_exit() {
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let confirmed = Arc::new(AtomicBool::new(false));
    lifecycle.cancel();
    let (backoff_tx, backoff_rx) = mpsc::sync_channel(READER_BACKOFF_CHANNEL_CAPACITY);
    let (complete_tx, complete_rx) = mpsc::sync_channel(READER_BACKOFF_CHANNEL_CAPACITY);
    lifecycle.install_restart_wait_probe(backoff_tx);
    let waiter_lifecycle = Arc::clone(&lifecycle);
    let waiter_confirmed = Arc::clone(&confirmed);
    let waiter = std::thread::spawn(move || {
        waiter_lifecycle.wait_for_restart(
            SUPERVISION_RESTART_MAX_DELAY,
            waiter_confirmed.as_ref(),
            ReaderRestartPhase::AwaitingChildExit,
            ReaderRestartKind::PendingDelivery,
        );
        complete_tx.send(()).unwrap();
    });

    backoff_rx
        .recv_timeout(READER_EXIT_RECOVERY_TIMEOUT)
        .expect(READER_BACKOFF_NOT_ENTERED_DETAIL);
    assert!(matches!(
        complete_rx.recv_timeout(READER_RESTART_WAKE_TIMEOUT),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    confirmed.store(true, Ordering::Release);
    lifecycle.notify_child_exit_confirmation();
    complete_rx
        .recv_timeout(READER_RESTART_WAKE_TIMEOUT)
        .unwrap();
    waiter.join().unwrap();
}
