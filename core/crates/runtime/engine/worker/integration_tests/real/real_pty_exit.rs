use crate::worker::tests::*;
const REAL_PTY_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const ROOT_FIRST_EXIT_TIMEOUT: Duration = Duration::from_secs(15);
const REAL_PTY_SHELL: &str = "/bin/sh";
const REAL_PTY_SHELL_INTERACTIVE_ARGUMENT: &str = "-i";
pub(super) const ROOT_FIRST_MARKER: &str = "root-first-child.pid";
const ROOT_FIRST_TRIGGER_EXTENSION: &str = "trigger";
const ROOT_FIRST_LAUNCH_EXTENSION: &str = "launched";
const ROOT_FIRST_INTERMEDIATE_LOCK_EXTENSION: &str = "intermediate-lock";
pub(super) const ROOT_FIRST_INTERMEDIATE_READY_EXTENSION: &str = "intermediate-ready";
pub(super) const ROOT_FIRST_INTERMEDIATE_RELEASE_EXTENSION: &str = "intermediate-release";
pub(super) const ROOT_FIRST_REOPEN_READY_EXTENSION: &str = "reopen-ready";
pub(super) const ROOT_FIRST_REOPEN_RELEASE_EXTENSION: &str = "reopen-release";
pub(super) const ROOT_FIRST_ROOT_RELEASE_EXTENSION: &str = "root-release";
pub(super) const EMPTY_MARKER: &[u8] = b"";
const ROOT_FIRST_HELPER_TEST: &str = "worker::tests::real_pty_exit::root_first_setsid_helper";
const ROOT_FIRST_HELPER_ROLE_ENV: &str = "YOLO_ROOT_FIRST_HELPER";
const ROOT_FIRST_HELPER_MARKER_ENV: &str = "YOLO_ROOT_FIRST_MARKER";
const ROOT_FIRST_LAUNCH_MARKER_ENV: &str = "YOLO_ROOT_FIRST_LAUNCH_MARKER";
const ROOT_FIRST_INTERMEDIATE_LOCK_ENV: &str = "YOLO_ROOT_FIRST_INTERMEDIATE_LOCK";
const ROOT_FIRST_INTERMEDIATE_READY_ENV: &str = "YOLO_ROOT_FIRST_INTERMEDIATE_READY";
const ROOT_FIRST_INTERMEDIATE_RELEASE_ENV: &str = "YOLO_ROOT_FIRST_INTERMEDIATE_RELEASE";
const ROOT_FIRST_REOPEN_READY_ENV: &str = "YOLO_ROOT_FIRST_REOPEN_READY";
const ROOT_FIRST_REOPEN_RELEASE_ENV: &str = "YOLO_ROOT_FIRST_REOPEN_RELEASE";
const ROOT_FIRST_TTY_ENV: &str = "YOLO_ROOT_FIRST_TTY";
const ROOT_FIRST_SKIP_REOPEN_ENV: &str = "YOLO_ROOT_FIRST_SKIP_REOPEN";
const ROOT_FIRST_SKIP_REOPEN_VALUE: &str = "1";
const ROOT_FIRST_INTERMEDIATE_ROLE: &str = "double-fork-intermediate";
const ROOT_FIRST_LEAF_ROLE: &str = "setsid-slave-holder";
const ROOT_FIRST_HELPER_SUCCESS_EXIT_CODE: i32 = 0;
const SETSID_FAILURE: libc::pid_t = -1;
const ROOT_FIRST_PROCESS_TIMEOUT_DETAIL: &str = "root-first helper did not publish its process id";
const ROOT_FIRST_READY_TIMEOUT_DETAIL: &str = "root-first helper did not reach its reopen gate";
#[cfg(target_os = "macos")]
const ROOT_FIRST_PTY_PROOF_DETAIL: &str = "root-first descendant did not retain its PTY proof";
const ROOT_FIRST_TERMINAL_DETAIL: &str = "root-first PTY output";

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum RootFirstLeafPtyMode {
    Reopen,
    Closed,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_pty_interactive_shell_exit_reaches_durable_finalization() {
    let mut command = CommandBuilder::new(REAL_PTY_SHELL);
    command.arg(REAL_PTY_SHELL_INTERACTIVE_ARGUMENT);
    let fixture = Fixture::with_contained_command(command);
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    fixture.spawn_reader(reader);
    let mut host_release = start_fixture_owner(&fixture);

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
        REAL_PTY_EXIT_TIMEOUT,
    )
    .await;
    if let Some(release) = host_release.as_mut() {
        release.wait();
    }
    assert!(
        read_json::<ExitRecord>(&fixture.state.paths.exit_record(fixture.state.terminal_id()))
            .is_ok()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn root_first_exit_cleans_slave_holder_before_finalization() {
    let directory = tempfile::tempdir().unwrap();
    let marker_path = directory.path().join(ROOT_FIRST_MARKER);
    let mut capability = FixturePtyCapability::queued_descriptor();
    capability.assert_queued_writer_present();
    let fixture = Fixture::with_contained_command(root_first_command(
        &marker_path,
        capability.value(),
        RootFirstLeafPtyMode::Reopen,
    ));
    let root = fixture_root_identity(&fixture);
    let root_pid = root.pid;
    #[cfg(target_os = "macos")]
    let root_kernel = process::kernel_process_metadata_for_test(root_pid).unwrap();
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    fixture.spawn_reader(reader);
    let mut owner = start_gated_root_owner(&fixture, &capability);
    release_root_first_trigger(&marker_path);
    let intermediate_ready = marker_path.with_extension(ROOT_FIRST_INTERMEDIATE_READY_EXTENSION);
    wait_for_marker_file(
        &intermediate_ready,
        &marker_path,
        &fixture.state,
        ROOT_FIRST_EXIT_TIMEOUT,
    )
    .await;
    let root_release = marker_path.with_extension(ROOT_FIRST_ROOT_RELEASE_EXTENSION);
    std::fs::write(root_release, EMPTY_MARKER).unwrap();
    owner.wait_for_root_exit(ROOT_FIRST_EXIT_TIMEOUT);
    capability.assert_synthetic_writer_present(&fixture);
    let intermediate_release =
        marker_path.with_extension(ROOT_FIRST_INTERMEDIATE_RELEASE_EXTENSION);
    std::fs::write(intermediate_release, EMPTY_MARKER).unwrap();
    let ready = marker_path.with_extension(ROOT_FIRST_REOPEN_READY_EXTENSION);
    wait_for_marker_file(
        &ready,
        &marker_path,
        &fixture.state,
        ROOT_FIRST_EXIT_TIMEOUT,
    )
    .await;
    let release = marker_path.with_extension(ROOT_FIRST_REOPEN_RELEASE_EXTENSION);
    std::fs::write(release, EMPTY_MARKER).unwrap();
    let descendant = wait_for_process_file(&marker_path, root_pid, ROOT_FIRST_EXIT_TIMEOUT).await;
    owner.retain_cleanup_identity(descendant);
    assert_eq!(
        unsafe { libc::getsid(descendant.pid as libc::pid_t) },
        descendant.pid as libc::pid_t
    );
    #[cfg(target_os = "macos")]
    {
        assert_root_first_leaf_pty_proof(&capability, &fixture, descendant);
        let leaf_kernel =
            wait_for_descendant_reparent(descendant, root_pid, ROOT_FIRST_EXIT_TIMEOUT);
        assert_ne!(leaf_kernel.parent_unique_id, root_kernel.unique_id);
    }
    assert!(!lock_recover(&fixture.state.reader_lifecycle.progress).is_drained());
    assert!(!fixture.state.child_waited.load(Ordering::Acquire));
    let manager_inspection = capability.inspection_proof(&fixture);
    let cleanup_path = owner.release_after_cleanup_started(ROOT_FIRST_EXIT_TIMEOUT);
    #[cfg(target_os = "macos")]
    assert_eq!(
        cleanup_path,
        RootExitCleanupPath::FaultReaperDeferred,
        "{FAULT_REAP_DEFERRED_PATH_DETAIL}"
    );
    owner.assert_retained_runtime_status(&fixture.state, cleanup_path);
    capability.release_writer_after_root_exit(&owner.root_live);
    capability.assert_manager_excluded(manager_inspection, ROOT_FIRST_EXIT_TIMEOUT);
    owner.wait_for_settlement(ROOT_FIRST_EXIT_TIMEOUT);
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
    assert_root_first_leaf_cleanup_won(&marker_path);
    assert!(fixture.state.reader_lifecycle.producers_are_gone());
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).is_drained());
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    assert_eq!(fixture.state.termination_reason(), None);
    let exit: ExitRecord =
        read_json(&fixture.state.paths.exit_record(fixture.terminal_id())).unwrap();
    assert_eq!(exit.reason, ExitReason::Natural);
}
include!("../../test_support/integration/real_pty/real_pty_fixture_owner.rs");
include!("../../test_support/integration/real_pty/real_pty_force_support.rs");
include!("real_pty_capability_holder.rs");
include!("../../test_support/integration/real_pty/real_pty_capability.rs");
include!("../../test_support/integration/real_pty/real_pty_capability_queue.rs");
include!("../../test_support/integration/real_pty/real_pty_host_release.rs");
include!("../../test_support/integration/real_pty/real_pty_reparent.rs");
include!("../../test_support/integration/real_pty/real_pty_force_progress.rs");
include!("../../test_support/integration/real_pty/real_pty_lineage_gate.rs");
include!("../../test_support/integration/real_pty/real_pty_reaper_state.rs");
include!("../../test_support/integration/real_pty/real_pty_lineage_cleanup.rs");
include!("../../test_support/integration/socket_namespace_settlement.rs");
include!("../../test_support/integration/real_pty/real_pty_leaf_lifetime.rs");
include!("real_pty_double_fork.rs");
include!("../../test_support/integration/real_pty/real_pty_helper_runtime.rs");
include!("../../test_support/integration/real_pty/real_pty_command.rs");
include!("../../test_support/integration/real_pty/real_pty_wait.rs");
include!("real_pty_tentative_eof.rs");
#[cfg(target_os = "macos")]
include!("real_pty_stopped_matrix.rs");
