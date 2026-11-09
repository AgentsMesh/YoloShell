const ACTIVE_FORCE_OUTPUT_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_force_termination_requests_reader_backed_output_progress() {
    let fixture = Fixture::with_contained_command(CommandBuilder::new(FIXTURE_COMMAND));
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    fixture.spawn_reader(reader);
    let output_calls = Arc::new(std::sync::atomic::AtomicUsize::new(usize::default()));
    let output = crate::worker::pty_output_progress::ReaderBackedPtyOutput::observed_for_runtime(
        Arc::clone(&output_calls),
        Arc::clone(&fixture.state.reader_lifecycle),
        Arc::clone(&fixture.state.child_waited),
    );
    let record = lock_recover(&fixture.state.record);
    let root = process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    drop(record);
    let mut host_release = fixture
        .startup
        .lock()
        .unwrap()
        .as_ref()
        .map(|_| GatedHostRelease::respond(&fixture));
    let mut guard = match fixture.startup.lock().unwrap().take() {
        Some(guard) => guard,
        None => {
            let child = fixture.child.lock().unwrap().take().unwrap();
            let mut guard = StartupChildGuard::with_reader_backed_output(child, output);
            guard.track(root).unwrap();
            guard
        }
    };
    pty_owner_handle::spawn_guarded_owner(&mut guard, Arc::clone(&fixture.state)).unwrap();
    #[cfg(target_os = "linux")]
    let previous_probe =
        lock_recover(&fixture.state.reader_lifecycle.progress).requested_probe_epoch;

    let permit = fixture.state.admit_system_mutation().await;
    fixture
        .state
        .begin_system_termination(&permit, ExitReason::UserTerminate);
    terminate_pty_processes(&fixture.state, permit, true)
        .await
        .unwrap();

    wait_for_runtime_status(
        &fixture.state,
        TerminalRuntimeStatus::Exited,
        ACTIVE_FORCE_OUTPUT_TIMEOUT,
    )
    .await;
    #[cfg(not(target_os = "linux"))]
    assert!(output_calls.load(Ordering::Acquire) > usize::default());
    #[cfg(target_os = "linux")]
    assert!(
        lock_recover(&fixture.state.reader_lifecycle.progress).requested_probe_epoch
            > previous_probe
    );
    if let Some(release) = host_release.as_mut() {
        release.wait();
    }
    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    assert_eq!(exit.reason, ExitReason::UserTerminate);
    assert_eq!(exit.signal, Some(libc::SIGKILL));
}
