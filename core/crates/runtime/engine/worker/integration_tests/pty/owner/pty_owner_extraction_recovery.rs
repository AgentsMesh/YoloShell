use crate::worker::tests::*;
const EXTRACTION_EVENT_TIMEOUT: Duration = OWNER_FAULT_TIMEOUT;
const EXTRACTION_DUPLICATE_WINDOW: Duration = Duration::from_millis(100);
const EXTRACTION_CHILD_POLL_INTERVAL: Duration = Duration::from_millis(5);
const EXTRACTION_EXIT_EVENT_MISSING_DETAIL: &str =
    "extraction recovery did not publish a durable exit event";
const EXTRACTION_DUPLICATE_EXIT_DETAIL: &str =
    "extraction recovery published more than one exit event";
const EXTRACTION_EXIT_RECORD_MISSING_DETAIL: &str =
    "extraction recovery did not persist an exit record";
const EXTRACTION_CHILD_REMAINED_DETAIL: &str = "extraction recovery left the child process running";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn worker_state_finalizes_after_each_extraction_panic() {
    for &phase in crate::worker::pty_ownership::HandoffExtractionPhase::ALL {
        recover_extraction_phase(phase).await;
    }
}

async fn recover_extraction_phase(phase: crate::worker::pty_ownership::HandoffExtractionPhase) {
    let fixture = Fixture::new();
    let mut events = fixture.state.events.subscribe();
    let root = extraction_fixture_root_identity(&fixture);
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    spawn_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let mut ownership = take_fault_ownership(&fixture);
    ownership.panic_after_handoff_extraction_for_test(phase);
    let (_commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner_settlement = Arc::clone(&settlement);
    let state = Arc::downgrade(&fixture.state);
    let owner = std::thread::spawn(move || {
        ownership
            .into_owner_for_test(
                state,
                Box::new(FaultingRootObserver),
                receiver,
                owner_settlement,
            )
            .run();
    });

    wait_for_runtime_status(&fixture.state, TerminalRuntimeStatus::Exited, OWNER_FAULT_TIMEOUT).await;
    let first_exit = receive_extraction_exit(&mut events).await;
    assert_eq!(first_exit.reason, ExitReason::Lost);
    assert!(
        fixture
            .state
            .exit_finalization_started
            .load(Ordering::Acquire)
    );
    assert_no_duplicate_extraction_exit(&mut events).await;
    let persisted: ExitRecord = read_json(&fixture.state.paths.exit_record(fixture.terminal_id()))
        .expect(EXTRACTION_EXIT_RECORD_MISSING_DETAIL);
    assert_eq!(persisted.runtime_epoch, first_exit.runtime_epoch);
    assert_eq!(persisted.reason, first_exit.reason);
    assert!(lock_recover(&fixture.state.exit).is_some());
    owner.join().unwrap();
    wait_for_extraction_child_gone(root);
    assert!(settlement.get().is_some());
}

fn extraction_fixture_root_identity(fixture: &Fixture) -> process::ProcessIdentity {
    let record = lock_recover(&fixture.state.record);
    process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    }
}

async fn receive_extraction_exit(
    events: &mut tokio::sync::broadcast::Receiver<WorkerEvent>,
) -> ExitRecord {
    let deadline = tokio::time::Instant::now() + EXTRACTION_EVENT_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let event = tokio::time::timeout(remaining, events.recv())
            .await
            .expect(EXTRACTION_EXIT_EVENT_MISSING_DETAIL)
            .expect(EXTRACTION_EXIT_EVENT_MISSING_DETAIL);
        if let WorkerEvent::Exited(exit) = event {
            return exit;
        }
    }
}

async fn assert_no_duplicate_extraction_exit(
    events: &mut tokio::sync::broadcast::Receiver<WorkerEvent>,
) {
    let deadline = tokio::time::Instant::now() + EXTRACTION_DUPLICATE_WINDOW;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return;
        }
        match tokio::time::timeout(remaining, events.recv()).await {
            Ok(Ok(WorkerEvent::Exited(_))) => panic!("{EXTRACTION_DUPLICATE_EXIT_DETAIL}"),
            Ok(Ok(_)) | Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {}
            Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) | Err(_) => return,
        }
    }
}

fn wait_for_extraction_child_gone(root: process::ProcessIdentity) {
    let deadline = Instant::now() + EXTRACTION_EVENT_TIMEOUT;
    while root.matches() {
        assert!(
            Instant::now() < deadline,
            "{EXTRACTION_CHILD_REMAINED_DETAIL}"
        );
        std::thread::sleep(EXTRACTION_CHILD_POLL_INTERVAL);
    }
}
