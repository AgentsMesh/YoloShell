const STOPPED_OUTPUT_FAULT_TIMEOUT: Duration = Duration::from_secs(8);
const STOPPED_OUTPUT_FAULT_EXPECTED_SIGNAL: i32 = libc::SIGHUP;
const STOPPED_OUTPUT_EXPECTED_DEFER_DETAIL: &str = "stopped-output fault reaped inline";
const STOPPED_OUTPUT_REAPER_STATUS_DETAIL: &str = "stopped-output fault reaper lost wait status";

async fn run_stopped_fault(context: &StoppedOutputContext) {
    let directory = tempfile::tempdir().unwrap();
    let progress = stopped_output_progress_path(directory.path());
    let ready = stopped_output_ready_path(directory.path());
    let release = stopped_output_release_path(directory.path());
    prepare_stopped_output_progress(&progress);
    let fixture = Fixture::with_command(stopped_output_writer_command(&progress));
    wait_for_stopped_output_ready(&ready);
    let output_control = {
        let master = lock_recover(&fixture.state.master);
        StoppedOutputGuard::prepare(master.as_ref().unwrap().as_ref(), context.case.mode).unwrap()
    };
    let root = fixture_root_identity(&fixture);
    context.publish_root(root).unwrap();
    let ownership = take_real_output_ownership(&fixture);
    release_stopped_output_writer(&release);
    wait_for_stopped_output_warmup(&fixture, &progress);
    prove_stopped_output_advance(&progress);
    prove_stopped_output_transcript(&fixture, &progress);
    output_control.stop().unwrap();
    prove_stopped_output_backpressure(&progress);

    let fault_state = Arc::clone(&fixture.state);
    tokio::task::spawn_blocking(move || {
        let mut ownership = ownership;
        fault_state.begin_lost_ownership_fault();
        let (completion, receiver) = pty_fault_reaper::FaultReapCompletion::owner_channel();
        let settlement = ownership.settle_after_fault(completion);
        assert!(
            matches!(settlement, pty_ownership::FaultSettlement::Deferred(_)),
            "{STOPPED_OUTPUT_EXPECTED_DEFER_DETAIL}"
        );
        let outcome = receiver
            .receive_timeout(STOPPED_OUTPUT_FAULT_TIMEOUT)
            .unwrap();
        assert_eq!(outcome.evidence, pty_ownership::ReapEvidence::Waited);
        let status = outcome.status.expect(STOPPED_OUTPUT_REAPER_STATUS_DETAIL);
        assert_eq!(status.code(), None);
        assert_eq!(status.signal(), Some(STOPPED_OUTPUT_FAULT_EXPECTED_SIGNAL));
        yolo_coverage_test_support::discard_aborted_process_profile(root.pid).unwrap();
        finalize_waited_child(&fault_state, status.code(), status.signal());
    })
    .await
    .unwrap();
    wait_for_runtime_status(
        &fixture.state,
        TerminalRuntimeStatus::Exited,
        STOPPED_OUTPUT_FAULT_TIMEOUT,
    )
    .await;
    let exit: ExitRecord =
        read_json(&fixture.state.paths.exit_record(fixture.terminal_id())).unwrap();
    assert_eq!(exit.reason, ExitReason::Lost);
    assert_eq!(exit.signal, Some(STOPPED_OUTPUT_FAULT_EXPECTED_SIGNAL));
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    prove_stopped_output_transcript(&fixture, &progress);
    context.assert_root_gone().unwrap();
    drop(output_control);
}

fn take_real_output_ownership(fixture: &Fixture) -> pty_ownership::PtyOwnership {
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    let pending_reader = stage_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let output = {
        let master = lock_recover(&fixture.state.master);
        crate::worker::pty_output_progress::ExclusivePtyOutput::capture(
            master.as_ref().unwrap().as_ref(),
            Arc::clone(&fixture.state.journal),
        )
        .unwrap()
    };
    let child = fixture.child.lock().unwrap().take().unwrap();
    let root = fixture_root_identity(fixture);
    let mut guard = StartupChildGuard::with_output(child, output);
    guard.track(root).unwrap();
    guard.activate_reader(pending_reader).unwrap();
    let ownership = guard.take_ownership().unwrap();
    guard.disarm_after_transfer();
    ownership
}
