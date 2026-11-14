use crate::host::tests::*;
#[tokio::test]
async fn timed_out_and_cancelled_scans_release_capacity_for_a_third_scan() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    let coordinator = Arc::new(ProviderProcessScanCoordinator::new());
    let first = blocking_scanner(&fixture);
    let second = blocking_scanner(&fixture);
    let first_root = fixture._agent_process.agent();
    let second_root = CompletionProviderProcess::spawn();

    let first_wait = spawn_scan(
        coordinator.clone(),
        first.0.clone(),
        first_root,
        PROVIDER_WAIT_TEST_TIMEOUT,
    );
    let second_wait = spawn_scan(
        coordinator.clone(),
        second.0.clone(),
        second_root.agent(),
        PROVIDER_WAIT_TEST_TIMEOUT,
    );
    tokio::join!(first.1.await_entry(), second.1.await_entry());
    second_wait.abort();
    assert!(second_wait.await.unwrap_err().is_cancelled());
    let _ = first_wait.await.unwrap();
    tokio::join!(first.1.await_settlement(), second.1.await_settlement());
    await_scan_coordinator_idle(&coordinator).await;

    let third_root = CompletionProviderProcess::spawn();
    let third_scanner = Arc::new(ScriptedProviderProcessScanner::stable(vec![
        fixture.valid_process(),
    ]));
    let third = coordinator
        .observe(
            third_scanner,
            third_root.agent(),
            HostOperationDeadline::after(PROVIDER_WAIT_LONG_TIMEOUT),
        )
        .await
        .unwrap();

    assert!(matches!(
        third.as_ref(),
        ProviderProcessScanOutcome::Observed(_)
    ));
    await_scan_coordinator_idle(&coordinator).await;
}

#[tokio::test]
async fn dropping_scan_coordinator_aborts_owned_scan_tasks() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    let coordinator = ProviderProcessScanCoordinator::new();
    let (scanner, blocking) = blocking_scanner(&fixture);
    let entry = coordinator.entry(
        scanner,
        fixture._agent_process.agent(),
        HostOperationDeadline::after(PROVIDER_WAIT_LONG_TIMEOUT),
    );
    blocking.await_entry().await;
    drop(entry);
    drop(coordinator);
    blocking.await_settlement().await;
}

#[tokio::test]
async fn consumed_provider_scan_releases_capacity_for_another_root() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    let first = fixture
        .host
        .provider_runtime_membership(
            &fixture.transaction,
            &fixture.runtime,
            &fixture.resumed_binding(),
            HostOperationDeadline::after(PROVIDER_WAIT_LONG_TIMEOUT),
        )
        .await;
    assert!(matches!(first, ProviderRuntimeMembership::Verified(_)));

    let other_root = CompletionProviderProcess::spawn();
    let mut other_runtime = fixture.runtime.clone();
    other_runtime.child_pid = other_root.agent().pid;
    other_runtime.child_start_time = other_root.agent().start_time;
    let second = fixture
        .host
        .provider_runtime_membership(
            &fixture.transaction,
            &other_runtime,
            &fixture.resumed_binding(),
            HostOperationDeadline::after(PROVIDER_WAIT_LONG_TIMEOUT),
        )
        .await;

    assert!(matches!(second, ProviderRuntimeMembership::Verified(_)));
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_EXPECTED_TWO_SCAN_CALLS
    );
    assert_eq!(
        fixture.host.provider_process_scans.available_permits(),
        MAX_CONCURRENT_PROVIDER_PROCESS_SCANS
    );
}

#[tokio::test]
async fn provider_scan_rejects_receipts_not_started_after_the_binding() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    let binding = fixture.resumed_binding();
    let before = binding
        .updated_at_ms
        .saturating_sub(COMPLETION_PROVIDER_TIMESTAMP_INCREMENT);
    for (started_at_ms, completed_at_ms) in [
        (before, before),
        (
            before,
            binding
                .updated_at_ms
                .saturating_add(COMPLETION_PROVIDER_TIMESTAMP_INCREMENT),
        ),
        (binding.updated_at_ms, binding.updated_at_ms),
    ] {
        fixture.set_scan(ScriptedProviderScan::ObservedAt(
            crate::agent::FencedDescendants::Stable(vec![fixture.valid_process()]),
            started_at_ms,
            completed_at_ms,
        ));
        let membership = fixture
            .host
            .provider_runtime_membership(
                &fixture.transaction,
                &fixture.runtime,
                &binding,
                HostOperationDeadline::after(PROVIDER_WAIT_LONG_TIMEOUT),
            )
            .await;
        assert_eq!(membership, ProviderRuntimeMembership::NotObserved);
    }
}

#[tokio::test]
async fn provider_scan_treats_an_empty_argv_snapshot_as_inconclusive() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    let mut process = fixture.valid_process();
    process.argv.clear();
    fixture.set_scan(ScriptedProviderScan::Stable(vec![process]));

    let membership = fixture
        .host
        .provider_runtime_membership(
            &fixture.transaction,
            &fixture.runtime,
            &fixture.resumed_binding(),
            HostOperationDeadline::after(PROVIDER_WAIT_LONG_TIMEOUT),
        )
        .await;

    assert_eq!(membership, ProviderRuntimeMembership::NotObserved);
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_EXPECTED_SCAN_CALLS
    );
}

#[tokio::test]
async fn provider_wait_retries_an_incomplete_snapshot_before_accepting_exact_authority() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    let binding = fixture.resumed_binding();
    fixture.publish_binding(&binding);
    let mut incomplete = fixture.valid_process();
    incomplete.argv.clear();
    incomplete.environment = None;
    fixture.set_scan(ScriptedProviderScan::sequence(vec![
        crate::agent::FencedDescendants::Stable(vec![incomplete]),
        crate::agent::FencedDescendants::Stable(vec![fixture.valid_process()]),
    ]));

    let authority = fixture
        .host
        .await_authoritative_resume_binding(
            &fixture.manifest,
            &fixture.transaction,
            &fixture.runtime,
        )
        .await
        .unwrap()
        .unwrap()
        .provider_completion
        .unwrap();

    assert_eq!(authority.binding, binding);
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_EXPECTED_TWO_SCAN_CALLS
    );
}
