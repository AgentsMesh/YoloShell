const STAGED_OWNER_CUSTODY_DETAIL: &str = "abandoned staged owner released the tracked PTY root";
const OWNER_CONSTRUCTION_PANIC_DETAIL: &str = "injected PTY owner construction panic";
const OWNER_CONSTRUCTION_SETTLEMENT_DETAIL: &str =
    "PTY owner construction panic did not publish settlement";
const OWNER_CONSTRUCTION_CUSTODY_DETAIL: &str =
    "PTY owner construction panic did not release the tracked root";

#[test]
fn abandoned_staged_owner_leaves_guard_custody_intact_before_activation() {
    let fixture = Fixture::new();
    let root = fixture_root_identity(&fixture);
    let output = {
        let master = lock_recover(&fixture.state.master);
        crate::worker::pty_output_progress::ExclusivePtyOutput::capture(
            master.as_ref().unwrap().as_ref(),
            Arc::clone(&fixture.state.journal),
        )
        .unwrap()
    };
    let child = fixture.child.lock().unwrap().take().unwrap();
    let mut guard = StartupChildGuard::with_output(child, output);
    guard.track(root).unwrap();

    let staged =
        pty_owner_handle::stage_guarded_owner(&mut guard, Arc::clone(&fixture.state)).unwrap();
    assert!(guard.fault_reaper.is_some());
    assert!(guard.untracked_reaper.is_some());
    drop(staged);
    assert!(guard.authority.is_some());
    assert!(guard.output_progress.is_some());
    drop(guard);

    let deadline = Instant::now() + WORKER_TERMINATION_TIMEOUT;
    while root.matches() {
        assert!(Instant::now() < deadline, "{STAGED_OWNER_CUSTODY_DETAIL}");
        std::thread::sleep(TEST_READY_POLL_INTERVAL);
    }
}

#[test]
fn construction_panic_cannot_publish_ready_or_settlement_before_custody_drop() {
    let fixture = Fixture::new();
    let root = fixture_root_identity(&fixture);
    fixture.state.ready.store(false, Ordering::Release);
    let child = fixture.child.lock().unwrap().take().unwrap();
    let mut guard =
        StartupChildGuard::with_reader_backed_output(child, fixture_runtime_output(&fixture));
    guard.track(root).unwrap();
    let mut staged =
        pty_owner_handle::stage_guarded_owner(&mut guard, Arc::clone(&fixture.state)).unwrap();
    let settlement = staged.settlement_for_test();
    let barrier = Arc::new(control_test_hooks::BlockingControlHook::default());
    barrier.arm();
    staged.inject_construction_panic(OWNER_CONSTRUCTION_PANIC_DETAIL);
    staged.block_publication_after_custody_drop(Arc::clone(&barrier));

    let activation = std::thread::spawn(move || staged.activate(&mut guard));
    barrier.wait_until_reached();

    assert!(settlement.get().is_none());
    assert!(fixture.state.pty_owner_handle().is_none());
    assert_eq!(fixture.state.status().status, TerminalRuntimeStatus::Starting);
    assert!(fixture.state.runtime_capabilities().is_empty());

    barrier.resume();
    let error = activation.join().unwrap().unwrap_err();
    assert!(error.to_string().contains(OWNER_CONSTRUCTION_PANIC_DETAIL));
    let completion = settlement
        .get()
        .expect(OWNER_CONSTRUCTION_SETTLEMENT_DETAIL);
    assert!(matches!(
        completion,
        Err(pty_owner_handle::PtyOwnerCommandFailure::Owner(_))
    ));
    assert!(fixture.state.pty_owner_handle().is_none());

    let deadline = Instant::now() + WORKER_TERMINATION_TIMEOUT;
    while root.matches() {
        assert!(
            Instant::now() < deadline,
            "{OWNER_CONSTRUCTION_CUSTODY_DETAIL}"
        );
        std::thread::sleep(TEST_READY_POLL_INTERVAL);
    }
}
