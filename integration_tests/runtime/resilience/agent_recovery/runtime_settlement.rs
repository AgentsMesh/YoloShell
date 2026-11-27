use super::*;

const AUTOMATIC_STOP_FAILURE_DETAIL: &str = "automatic recovery runtime termination failed";
const WORKER_SETTLEMENT_BUDGET: Duration =
    yolo_session_timing::WORKER_SHUTDOWN_SETTLEMENT_TIMEOUT.saturating_add(PROCESS_EXIT_TIMEOUT);

pub(super) async fn stop_automatic_runtime(
    client: &Client,
    fixture: &RecoveryFixture,
    automatic: &TerminalView,
    harness: &Harness,
) {
    if matches!(
        fixture.expectation,
        AutomaticExpectation::ExplicitOnly | AutomaticExpectation::FailClosed
    ) {
        return;
    }
    let terminal_id = fixture.previous.manifest.terminal_id;
    if let Err(error) = client.terminate(terminal_id, false).await {
        let session_view = client.get_session(terminal_id).await;
        panic!(
            "{AUTOMATIC_STOP_FAILURE_DETAIL}: {error:?}; session_view={session_view:?}; host_stderr={}",
            harness.host_stderr_tail()
        );
    }
    wait_for_exit(client, terminal_id).await;
    let runtime = automatic.runtime.as_ref().expect("automatic runtime");
    let worker = ProcessIdentity {
        pid: runtime.worker_pid,
        start_time: runtime.worker_start_time,
    };
    let child = ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    };
    tokio::join!(
        wait_for_identity_gone_with_budget(worker, WORKER_SETTLEMENT_BUDGET),
        wait_for_identity_gone(child)
    );
}
