use crate::host::tests::*;
#[tokio::test]
async fn an_expired_provider_scan_cancels_before_polling_the_scanner() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    let coordinator = ProviderProcessScanCoordinator::new();
    let scanner = Arc::new(ScriptedProviderProcessScanner::stable(vec![
        fixture.valid_process(),
    ]));
    let root = fixture._agent_process.agent();
    let entry = coordinator.entry(scanner.clone(), root, HostOperationDeadline::after(Duration::ZERO));
    let result = coordinator
        .observe(scanner.clone(), root, HostOperationDeadline::after(Duration::ZERO))
        .await;
    assert!(result.is_none());
    drop(entry);
    tokio::task::yield_now().await;
    await_scan_coordinator_idle(&coordinator).await;
    assert_eq!(scanner.calls(), COMPLETION_PROVIDER_NO_SCAN_CALLS);
    assert_eq!(coordinator.available_permits(), MAX_CONCURRENT_PROVIDER_PROCESS_SCANS);
}
