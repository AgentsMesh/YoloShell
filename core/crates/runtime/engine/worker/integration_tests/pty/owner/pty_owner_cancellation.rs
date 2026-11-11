use crate::worker::tests::*;
include!("../../../test_support/integration/pty/pty_owner_cancellation_support.rs");
include!("pty_owner_cancellation_settlement.rs");
include!("../../../test_support/integration/pty/pty_owner_cancellation_child.rs");
include!("pty_owner_cancellation_containment_tests.rs");
include!("../../../test_support/integration/pty/pty_owner_cancellation_watchdog.rs");
include!("../../../test_support/integration/pty/pty_owner_cancellation_result.rs");

const OWNER_CANCELLATION_SETUP_TIMEOUT: Duration = Duration::from_secs(8);
const OWNER_CANCELLATION_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const OWNER_CANCELLATION_ROOT_EXIT_TIMEOUT: Duration = Duration::from_secs(8);
const OWNER_CANCELLATION_HANDLER_ABORT_TIMEOUT: Duration = Duration::from_secs(1);
const OWNER_CANCELLATION_SETTLEMENT_TIMEOUT: Duration = FORCE_SETTLEMENT_INTEGRATION_TIMEOUT;
const OWNER_CANCELLATION_RUNTIME_TIMEOUT: Duration = Duration::from_secs(10);
const OWNER_CANCELLATION_ISOLATION_ENV: &str = "YOLO_OWNER_CANCELLATION_ISOLATION";
const OWNER_CANCELLATION_ISOLATION_ROLE: &str = "run-isolated-worker";
const OWNER_CANCELLATION_ISOLATED_TEST: &str =
    "worker::tests::pty_owner_cancellation::dropped_termination_future_isolated_worker";
const OWNER_CANCELLATION_HELPER_FAILURE: &str = "isolated owner cancellation worker failed";
const OWNER_CANCELLATION_FAILURE_SEPARATOR: &str = "; ";
const OWNER_CANCELLATION_REQUEST_ID: &str = "owner-cancellation-termination";
const OWNER_CANCELLATION_REQUEST_DETAIL: &str =
    "termination request did not become durable before its deadline";
const OWNER_CANCELLATION_HANDLER_PENDING_DETAIL: &str =
    "termination handler completed before owner cleanup";
const OWNER_CANCELLATION_HANDLER_ABORT_DETAIL: &str =
    "termination handler did not cancel before its deadline";
const OWNER_CANCELLATION_MUTATION_GATE_DETAIL: &str =
    "termination handler retained the mutation gate after command acceptance";

#[test]
fn dropped_termination_future_does_not_cancel_accepted_cleanup() {
    run_owner_cancellation_isolation().unwrap();
}

fn run_owner_cancellation_isolation() -> Result<()> {
    let mut control = OwnerCancellationControl::create()?;
    let mut socket_root = crate::paths::IsolatedTestSocketRoot::create()?;
    let mut failures = Vec::new();
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command.args([
        BOOTSTRAP_TEST_EXACT_ARGUMENT,
        OWNER_CANCELLATION_ISOLATED_TEST,
        BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
        crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
    ]);
    command.env(
        OWNER_CANCELLATION_ISOLATION_ENV,
        OWNER_CANCELLATION_ISOLATION_ROLE,
    );
    command.env(crate::paths::SOCKET_ROOT_ENVIRONMENT, socket_root.path());
    command.stdin(std::process::Stdio::null());
    control.configure(&mut command);

    match OwnerCancellationChild::spawn(command) {
        Ok(mut child) => match child.wait_for_exit() {
            Ok(status) if status.success() => {}
            Ok(status) => failures.push(format!("{OWNER_CANCELLATION_HELPER_FAILURE}: {status}")),
            Err(error) => {
                failures.push(error.to_string());
                if let Err(error) = child.terminate_and_reap() {
                    failures.push(error.to_string());
                }
            }
        },
        Err(error) => failures.push(error.to_string()),
    }
    record_owner_cancellation_result(&mut failures, control.verify());
    record_owner_cancellation_result(&mut failures, control.settle());
    record_owner_cancellation_result(
        &mut failures,
        settle_worker_test_socket_namespaces(&socket_root),
    );
    record_owner_cancellation_result(&mut failures, socket_root.finalize());
    anyhow::ensure!(
        failures.is_empty(),
        failures.join(OWNER_CANCELLATION_FAILURE_SEPARATOR)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropped_termination_future_isolated_worker() {
    if std::env::var(OWNER_CANCELLATION_ISOLATION_ENV).as_deref()
        != Ok(OWNER_CANCELLATION_ISOLATION_ROLE)
    {
        return;
    }
    let context = OwnerCancellationContext::required().unwrap();
    context
        .write_stage(OwnerCancellationStage::Started)
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(ROOT_FIRST_MARKER);
    let mut capability = FixturePtyCapability::queued_descriptor();
    capability.assert_queued_writer_present();
    let fixture = Fixture::with_contained_command(root_first_command(
        &marker,
        capability.value(),
        RootFirstLeafPtyMode::Reopen,
    ));
    let root = fixture_root_identity(&fixture);
    context
        .publish(OwnerCancellationHelperRole::Root, root)
        .unwrap();
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    fixture.spawn_reader(reader);
    let mut gated_owner = start_gated_root_owner(&fixture, &capability);
    let previous = lock_recover(&fixture.state.pty_owner).replace(gated_owner.handle());
    assert!(previous.is_none());
    prepare_owner_cancellation_leaf(&marker, &fixture).await;
    let descendant = wait_for_process_file(
        &marker,
        fixture.child_pid(),
        OWNER_CANCELLATION_SETUP_TIMEOUT,
    )
    .await;
    gated_owner.retain_cleanup_identity(descendant);

    let (mut client, handler) =
        authenticated_worker_connection(&fixture, FIXTURE_CONTROLLER_GENERATION).await;
    let hooks = &fixture.state.control_test_hooks;
    hooks.termination_after_tombstone.arm();
    let request = RequestEnvelope::new(
        OWNER_CANCELLATION_REQUEST_ID,
        method::TERMINATE_RUNTIME_EXACT,
        to_value(&FencedTerminateRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: fixture.state.runtime_epoch(),
            expected_status: None,
            force: false,
        })
        .unwrap(),
    );
    let invocation = tokio::spawn(async move { client.exchange(&request).await });
    let tombstone = wait_for_owner_cancellation_request(&fixture).await;
    hooks.termination_after_tombstone.wait_until_reached().await;
    assert!(
        !handler.is_finished(),
        "{OWNER_CANCELLATION_HANDLER_PENDING_DETAIL}"
    );
    let mutation = Arc::clone(&fixture.state.mutation_gate)
        .try_lock_owned()
        .expect(OWNER_CANCELLATION_MUTATION_GATE_DETAIL);
    drop(mutation);
    handler.abort();
    let cancellation = tokio::time::timeout(OWNER_CANCELLATION_HANDLER_ABORT_TIMEOUT, handler)
        .await
        .expect(OWNER_CANCELLATION_HANDLER_ABORT_DETAIL)
        .expect_err(OWNER_CANCELLATION_HANDLER_ABORT_DETAIL);
    assert!(cancellation.is_cancelled());
    invocation.abort();
    let _ = invocation.await;
    std::fs::write(
        marker.with_extension(ROOT_FIRST_ROOT_RELEASE_EXTENSION),
        EMPTY_MARKER,
    )
    .unwrap();
    settle_cancelled_owner_cleanup(&fixture, &mut gated_owner, &mut capability);
    wait_for_runtime_status(
        &fixture.state,
        TerminalRuntimeStatus::Exited,
        OWNER_CANCELLATION_RUNTIME_TIMEOUT,
    )
    .await;

    assert_owner_cancellation_identity_gone(root);
    assert_owner_cancellation_identity_gone(descendant);
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    assert!(fixture.state.reader_lifecycle.producers_are_gone());
    assert!(
        fixture
            .state
            .exit_finalization_started
            .load(Ordering::Acquire)
    );
    assert!(!fixture.state.output_blocked());
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).is_drained());
    assert_eq!(
        fixture.state.termination_reason(),
        Some(ExitReason::UserTerminate)
    );
    assert!(!tombstone.force);
    let exit: ExitRecord =
        read_json(&fixture.state.paths.exit_record(fixture.terminal_id())).unwrap();
    assert_eq!(exit.reason, ExitReason::UserTerminate);
    context
        .write_stage(OwnerCancellationStage::Completed)
        .unwrap();
}
