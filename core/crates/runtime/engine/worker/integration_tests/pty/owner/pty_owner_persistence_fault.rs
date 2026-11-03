#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn provisional_persist_failure_cannot_block_child_reap() {
    let fixture = Fixture::new();
    let root = fixture.state.pty_root_identity();
    fixture.state.control_test_hooks.block_exit_persist();
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    spawn_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let ownership = take_fault_ownership(&fixture);
    let (_commands, receiver) = std::sync::mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let state = Arc::downgrade(&fixture.state);
    let trace_events = Arc::new(std::sync::atomic::AtomicUsize::default());
    let trace_payload_dropped = Arc::new(AtomicBool::default());
    let subscriber = OwnerContainmentSubscriber {
        events: Arc::clone(&trace_events),
        payload_dropped: Arc::clone(&trace_payload_dropped),
    };
    let owner = std::thread::spawn(move || {
        let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
        tracing::subscriber::with_default(subscriber, || {
            ownership
                .into_owner_for_test(state, Box::new(FaultingRootObserver), receiver, settlement)
                .run();
        });
    });

    tokio::time::timeout(OWNER_FAULT_TIMEOUT, async {
        while !fixture.state.child_waited.load(Ordering::Acquire) {
            tokio::time::sleep(STARTUP_CHILD_EXIT_POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap();
    assert!(!root.matches());
    assert!(lock_recover(&fixture.state.exit).is_none());

    tokio::time::timeout(OWNER_FAULT_TIMEOUT, async {
        while !owner.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    owner.join().unwrap();
    assert!(trace_events.load(Ordering::SeqCst) > 0);
    assert!(!trace_payload_dropped.load(Ordering::SeqCst));

    fixture.state.control_test_hooks.allow_exit_persist();
    wait_for_runtime_status(&fixture.state, TerminalRuntimeStatus::Exited, OWNER_FAULT_TIMEOUT).await;
    let durable: ExitRecord =
        read_json(&fixture.state.paths.exit_record(fixture.terminal_id())).unwrap();
    assert_eq!(durable.reason, ExitReason::Lost);
}
