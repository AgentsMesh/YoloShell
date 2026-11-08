const TENTATIVE_EOF_TIMEOUT_DETAIL: &str =
    "reader did not observe tentative EOF while the descendant remained live";

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_descendant_without_a_pty_slave_cannot_deadlock_force_settlement() {
    run_non_pty_retirement().await;
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn linux_non_pty_member_defers_cleanup_until_its_owner_ends_it() {
    run_non_pty_retirement().await;
}

async fn run_non_pty_retirement() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(ROOT_FIRST_MARKER);
    let mut capability = FixturePtyCapability::queued_descriptor();
    capability.assert_queued_writer_present();
    let fixture = Fixture::with_contained_command(root_first_command(
        &marker,
        capability.value(),
        RootFirstLeafPtyMode::Closed,
    ));
    let root = fixture_root_identity(&fixture);
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    fixture.spawn_reader(reader);
    let mut owner = start_gated_root_owner(&fixture, &capability);
    release_root_first_trigger(&marker);
    let intermediate_ready = marker.with_extension(ROOT_FIRST_INTERMEDIATE_READY_EXTENSION);
    wait_for_marker_file(
        &intermediate_ready,
        &marker,
        &fixture.state,
        ROOT_FIRST_EXIT_TIMEOUT,
    )
    .await;
    std::fs::write(
        marker.with_extension(ROOT_FIRST_ROOT_RELEASE_EXTENSION),
        EMPTY_MARKER,
    )
    .unwrap();
    owner.wait_for_root_exit(ROOT_FIRST_EXIT_TIMEOUT);
    capability.assert_synthetic_writer_present(&fixture);
    let manager_inspection = capability.inspection_proof(&fixture);
    capability.assert_manager_excluded(manager_inspection, ROOT_FIRST_EXIT_TIMEOUT);
    std::fs::write(
        marker.with_extension(ROOT_FIRST_INTERMEDIATE_RELEASE_EXTENSION),
        EMPTY_MARKER,
    )
    .unwrap();
    let reopen_ready = marker.with_extension(ROOT_FIRST_REOPEN_READY_EXTENSION);
    wait_for_marker_file(
        &reopen_ready,
        &marker,
        &fixture.state,
        ROOT_FIRST_EXIT_TIMEOUT,
    )
    .await;
    std::fs::write(
        marker.with_extension(ROOT_FIRST_REOPEN_RELEASE_EXTENSION),
        EMPTY_MARKER,
    )
    .unwrap();
    let descendant = wait_for_process_file(&marker, root.pid, ROOT_FIRST_EXIT_TIMEOUT).await;
    let _retained_descendant = owner.retain_cleanup_identity(descendant);
    wait_for_tentative_eof(&fixture.state).await;
    assert!(descendant.matches());
    assert_eq!(
        fixture.state.reader_lifecycle.output_progress_state(),
        crate::worker::pty_output_progress::OutputProgressState::Pending
    );
    let cleanup_path = owner.release_after_cleanup_started(FORCE_SETTLEMENT_INTEGRATION_TIMEOUT);
    owner.assert_retained_runtime_status(&fixture.state, cleanup_path);
    capability.release_writer_after_root_exit(&owner.root_live);
    #[cfg(target_os = "linux")]
    finish_linux_retained_fixture(&owner, &fixture, &_retained_descendant);
    owner.wait_for_cleanup_completion(cleanup_path, FORCE_SETTLEMENT_INTEGRATION_TIMEOUT);
    assert_fault_reap_runtime_completed(&fixture.state, cleanup_path);

    wait_for_runtime_status(
        &fixture.state,
        TerminalRuntimeStatus::Exited,
        ROOT_FIRST_EXIT_TIMEOUT,
    )
    .await;
    owner.join();

    assert!(!root.matches());
    assert!(!descendant.matches());
    assert_root_first_leaf_cleanup_won(&marker);
    assert!(fixture.state.reader_lifecycle.producers_are_gone());
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).is_drained());
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    assert_eq!(fixture.state.termination_reason(), None);
    let exit: ExitRecord =
        read_json(&fixture.state.paths.exit_record(fixture.terminal_id())).unwrap();
    assert_eq!(exit.reason, ExitReason::Natural);
}

#[cfg(target_os = "linux")]
include!("../../test_support/integration/linux/linux_non_pty_retirement.rs");

async fn wait_for_tentative_eof(state: &WorkerState) {
    tokio::time::timeout(ROOT_FIRST_EXIT_TIMEOUT, async {
        loop {
            if lock_recover(&state.reader_lifecycle.progress)
                .eof
                .is_awaiting_publication()
            {
                return;
            }
            tokio::time::sleep(PROCESS_DISCOVERY_POLL_INTERVAL).await;
        }
    })
    .await
    .expect(TENTATIVE_EOF_TIMEOUT_DETAIL);
}
