const STOPPED_OUTPUT_STARTUP_REAP_DETAIL: &str = "startup cleanup did not reap stopped PTY root";
const STOPPED_OUTPUT_STARTUP_STATUS_DETAIL: &str =
    "startup cleanup did not retain stopped PTY wait status";
const STOPPED_OUTPUT_UNEXPECTED_EXIT_DETAIL: &str = "startup cleanup created an ExitRecord";
const STOPPED_OUTPUT_EXCLUSIVE_AUTHORITY_DETAIL: &str =
    "startup stopped-output authority is not exclusive";
const STOPPED_OUTPUT_STARTUP_EXPECTED_SIGNAL: i32 = libc::SIGKILL;
const STOPPED_OUTPUT_STARTUP_PROGRESS_TIMEOUT: Duration = Duration::from_secs(15);
const STOPPED_OUTPUT_STARTUP_SETTLEMENT_TIMEOUT: Duration =
    crate::worker::termination_timing::WORKER_TERMINATION_TIMEOUT;

fn run_stopped_startup(context: &StoppedOutputContext) {
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
    let observer = guard.fault_reaper.as_ref().unwrap().state_observer();
    let completion = guard
        .fault_reaper
        .as_mut()
        .unwrap()
        .observe_startup_completion();
    let terminal_id = fixture.terminal_id();
    let runtime_record = fixture.state.paths.runtime_record(terminal_id);
    {
        let mut record = lock_recover(&fixture.state.record);
        record.bootstrap_delivery = BootstrapDelivery::Failed;
        persist_bootstrap_record(&fixture.state.namespace, &runtime_record, &*record).unwrap();
    }
    let before_cleanup: RuntimeRecord = read_json(&runtime_record).unwrap();
    assert_eq!(before_cleanup.bootstrap_delivery, BootstrapDelivery::Failed);
    release_stopped_output_writer(&release);
    drive_startup_output_until_warm(&mut guard, &progress);
    prove_startup_output_advance(&mut guard, &progress);
    output_control.stop().unwrap();
    prove_stopped_output_backpressure(&progress);
    drop(guard);
    let outcome = completion
        .receive_timeout(STOPPED_OUTPUT_STARTUP_SETTLEMENT_TIMEOUT)
        .unwrap_or_else(|error| {
            panic!(
                "{STOPPED_OUTPUT_STARTUP_REAP_DETAIL}: {error:?}, state={:?}, status={:?}",
                observer.latest(),
                observer.status(),
            )
        });
    assert_eq!(outcome.evidence, pty_ownership::ReapEvidence::Waited);
    assert_eq!(
        outcome
            .status
            .expect(STOPPED_OUTPUT_STARTUP_STATUS_DETAIL)
            .signal(),
        Some(STOPPED_OUTPUT_STARTUP_EXPECTED_SIGNAL)
    );
    context
        .assert_root_gone()
        .expect(STOPPED_OUTPUT_STARTUP_REAP_DETAIL);
    yolo_coverage_test_support::discard_aborted_process_profile(root.pid).unwrap();
    let after_cleanup: RuntimeRecord = read_json(&runtime_record).unwrap();
    assert_eq!(after_cleanup.bootstrap_delivery, BootstrapDelivery::Failed);
    assert!(
        !fixture.state.paths.exit_record(terminal_id).exists(),
        "{STOPPED_OUTPUT_UNEXPECTED_EXIT_DETAIL}"
    );
    prove_stopped_output_transcript(&fixture, &progress);
    drop(output_control);
}

fn drive_startup_output_until_warm(guard: &mut StartupChildGuard, path: &Path) {
    let deadline = Instant::now() + STOPPED_OUTPUT_STARTUP_PROGRESS_TIMEOUT;
    loop {
        let progress = read_stopped_output_progress(path).unwrap();
        if progress >= STOPPED_OUTPUT_WARM_BYTES {
            return;
        }
        progress_startup_output(guard);
        let progress = read_stopped_output_progress(path).unwrap();
        if progress >= STOPPED_OUTPUT_WARM_BYTES {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{STOPPED_OUTPUT_WARMUP_DETAIL}: progress={progress}, required={STOPPED_OUTPUT_WARM_BYTES}"
        );
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn prove_startup_output_advance(guard: &mut StartupChildGuard, path: &Path) {
    let initial = read_stopped_output_progress(path).unwrap();
    let deadline = Instant::now() + STOPPED_OUTPUT_STARTUP_PROGRESS_TIMEOUT;
    loop {
        progress_startup_output(guard);
        if read_stopped_output_progress(path).unwrap() > initial {
            return;
        }
        assert!(Instant::now() < deadline, "{STOPPED_OUTPUT_ADVANCE_DETAIL}");
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn progress_startup_output(guard: &mut StartupChildGuard) {
    let Some(crate::worker::startup::startup_guard::StartupOutputProgress::Exclusive(output)) =
        guard.output_progress.as_mut()
    else {
        panic!("{STOPPED_OUTPUT_EXCLUSIVE_AUTHORITY_DETAIL}")
    };
    output.progress().unwrap();
}
