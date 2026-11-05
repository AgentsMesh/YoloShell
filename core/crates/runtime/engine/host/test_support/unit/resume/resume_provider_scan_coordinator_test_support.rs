use crate::host::tests::*;
fn blocking_scanner(
    fixture: &ProviderWaitFixture,
) -> (
    Arc<ScriptedProviderProcessScanner>,
    Arc<BlockingProviderScan>,
) {
    let blocking = BlockingProviderScan::new(crate::agent::FencedDescendants::Stable(vec![
        fixture.valid_process(),
    ]));
    let scanner = Arc::new(ScriptedProviderProcessScanner::stable(Vec::new()));
    scanner.set_scan(ScriptedProviderScan::Blocking(blocking.clone()));
    (scanner, blocking)
}

fn spawn_scan(
    coordinator: Arc<ProviderProcessScanCoordinator>,
    scanner: Arc<dyn crate::agent::ProcessScanner>,
    root: process::ProcessIdentity,
    timeout: Duration,
) -> tokio::task::JoinHandle<Option<Arc<ProviderProcessScanOutcome>>> {
    tokio::spawn(async move {
        coordinator
            .observe(scanner, root, HostOperationDeadline::after(timeout))
            .await
    })
}

async fn await_scan_coordinator_idle(coordinator: &ProviderProcessScanCoordinator) {
    tokio::time::timeout(PROVIDER_WAIT_IMMEDIATE_BOUND, async {
        loop {
            if coordinator.available_permits() == MAX_CONCURRENT_PROVIDER_PROCESS_SCANS
                && coordinator.entry_count() == COMPLETION_PROVIDER_NO_SCAN_CALLS
                && coordinator.task_count() == COMPLETION_PROVIDER_NO_SCAN_CALLS
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect(PROVIDER_BLOCKING_SCAN_SETTLEMENT_TIMEOUT_DETAIL);
}
