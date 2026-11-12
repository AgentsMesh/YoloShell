const STOPPED_OUTPUT_ACTIVE_REQUEST_TIMEOUT: Duration = WORKER_TERMINATION_TIMEOUT;
const STOPPED_OUTPUT_ACTIVE_SIGNAL_DETAIL: &str = "force termination did not record SIGKILL";
const STOPPED_OUTPUT_ACTIVE_TERMINATION_DETAIL: &str =
    "active stopped-output force termination failed";

async fn run_stopped_active(context: &StoppedOutputContext) {
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
    start_real_output_owner(&fixture);
    release_stopped_output_writer(&release);
    wait_for_stopped_output_warmup(&fixture, &progress);
    prove_stopped_output_advance(&progress);
    prove_stopped_output_transcript(&fixture, &progress);
    output_control.stop().unwrap();
    prove_stopped_output_backpressure(&progress);

    let permit = fixture.state.admit_system_mutation().await;
    fixture
        .state
        .begin_system_termination(&permit, ExitReason::UserTerminate);
    let cleanup_path = tokio::time::timeout(
        STOPPED_OUTPUT_ACTIVE_REQUEST_TIMEOUT,
        terminate_pty_processes(&fixture.state, permit, true),
    )
    .await
    .expect(STOPPED_OUTPUT_ACTIVE_TERMINATION_DETAIL);
    let cleanup_path = active_force_cleanup_path(cleanup_path).unwrap();
    wait_for_runtime_status(
        &fixture.state,
        TerminalRuntimeStatus::Exited,
        FORCE_SETTLEMENT_INTEGRATION_TIMEOUT,
    )
    .await;
    assert_fault_reap_runtime_completed(&fixture.state, cleanup_path);
    let exit: ExitRecord =
        read_json(&fixture.state.paths.exit_record(fixture.terminal_id())).unwrap();
    assert_eq!(exit.reason, ExitReason::UserTerminate);
    assert_eq!(exit.code, None);
    assert_eq!(
        exit.signal,
        Some(libc::SIGKILL),
        "{STOPPED_OUTPUT_ACTIVE_SIGNAL_DETAIL}: {exit:?}"
    );
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    yolo_coverage_test_support::discard_aborted_process_profile(root.pid).unwrap();
    prove_stopped_output_transcript(&fixture, &progress);
    context.assert_root_gone().unwrap();
    drop(output_control);
}

fn active_force_cleanup_path(result: Result<()>) -> Result<RootExitCleanupPath> {
    match result {
        Ok(()) => Ok(RootExitCleanupPath::InlineComplete),
        Err(error) if error.is::<CapabilitySettlementDeferred>() => {
            Ok(RootExitCleanupPath::FaultReaperDeferred)
        }
        Err(error) => Err(error).context(STOPPED_OUTPUT_ACTIVE_TERMINATION_DETAIL),
    }
}

#[test]
fn typed_active_force_deferral_uses_the_fault_reaper_path() {
    let deferred = anyhow::Error::new(CapabilitySettlementDeferred);
    assert_eq!(
        active_force_cleanup_path(Err(deferred)).unwrap(),
        RootExitCleanupPath::FaultReaperDeferred
    );
}

fn start_real_output_owner(fixture: &Fixture) {
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
    let staged_owner =
        pty_owner_handle::stage_guarded_owner(&mut guard, Arc::clone(&fixture.state)).unwrap();
    guard.activate_reader(pending_reader).unwrap();
    staged_owner.activate(&mut guard).unwrap();
}
