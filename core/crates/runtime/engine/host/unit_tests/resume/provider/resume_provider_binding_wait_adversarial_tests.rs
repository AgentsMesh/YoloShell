use crate::host::tests::*;
#[tokio::test]
async fn provider_wait_rejects_runtime_record_drift_during_publication() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    fixture.publish_binding(&fixture.planned);
    let runtime_path = fixture
        .host
        .paths
        .runtime_record(fixture.transaction.terminal_id);
    let mut drifted = fixture.runtime.clone();
    drifted.started_at_ms = drifted
        .started_at_ms
        .saturating_add(COMPLETION_PROVIDER_TIMESTAMP_INCREMENT);
    let wait = tokio::time::timeout(
        PROVIDER_WAIT_IMMEDIATE_BOUND,
        fixture.host.await_authoritative_resume_binding(
            &fixture.manifest,
            &fixture.transaction,
            &fixture.runtime,
        ),
    );
    let drift = async { atomic_write_json(&runtime_path, &drifted).unwrap() };
    let (result, ()) = tokio::join!(biased; wait, drift);
    let error = result
        .expect(PROVIDER_RUNTIME_DRIFT_TIMEOUT_DETAIL)
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(RESUME_AUTHENTICATED_RUNTIME_MISMATCH)
    );
    assert!(!error.is_pending());
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_NO_SCAN_CALLS
    );
    fixture.assert_authority_files_retained();
}

#[test]
fn provider_wait_requires_authority_for_a_normalized_raw_binding() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let mut unknown = fixture.resumed_binding();
    unknown.state = crate::agent::AgentState::Unknown;
    unknown.recovery_availability = crate::agent::AgentRecoveryAvailability::Available;
    fixture.publish_raw_binding(&unknown);

    let normalized = fixture
        .host
        .load_agent_binding(&fixture.manifest)
        .unwrap()
        .unwrap();
    assert_eq!(normalized.state, crate::agent::AgentState::Unknown);
    assert_eq!(
        normalized.recovery_availability,
        crate::agent::AgentRecoveryAvailability::Unavailable
    );
    assert!(
        !fixture
            .host
            .provider_startup_receipt_ready(&fixture.runtime, &unknown)
            .unwrap()
    );
    assert_eq!(
        fixture
            .host
            .observe_resume_provider_binding(
                &fixture.manifest,
                &fixture.transaction,
                &fixture.runtime,
            )
            .unwrap(),
        ProviderBindingObservation::Missing
    );
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_NO_SCAN_CALLS
    );
    fixture.assert_authority_files_retained();
}

#[tokio::test(start_paused = true)]
async fn provider_wait_treats_inconclusive_process_scans_as_pending() {
    for scan in [
        ScriptedProviderScan::Stable(Vec::new()),
        ScriptedProviderScan::RootChanged,
        ScriptedProviderScan::Failed,
    ] {
        let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
        fixture.publish_binding(&fixture.resumed_binding());
        fixture.set_scan(scan);
        let started = tokio::time::Instant::now();
        let error = fixture
            .host
            .await_authoritative_resume_binding(
                &fixture.manifest,
                &fixture.transaction,
                &fixture.runtime,
            )
            .await
            .unwrap_err();

        assert!(started.elapsed() >= PROVIDER_WAIT_TEST_TIMEOUT);
        assert!(error.to_string().contains(RESUME_PROVIDER_BINDING_PENDING));
        assert!(error.is_pending());
        assert!(fixture.process_scanner.calls() > COMPLETION_PROVIDER_NO_SCAN_CALLS);
        fixture.assert_authority_files_retained();
    }
}

#[tokio::test]
async fn provider_wait_treats_unavailable_environment_as_pending() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    fixture.publish_binding(&fixture.resumed_binding());
    let mut process = fixture.valid_process();
    process.environment = None;
    fixture.set_scan(ScriptedProviderScan::Stable(vec![process]));
    let error = fixture
        .host
        .await_authoritative_resume_binding(
            &fixture.manifest,
            &fixture.transaction,
            &fixture.runtime,
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains(RESUME_PROVIDER_BINDING_PENDING));
    assert!(error.is_pending());
    assert!(fixture.process_scanner.calls() > COMPLETION_PROVIDER_NO_SCAN_CALLS);
    fixture.assert_authority_files_retained();
}

#[tokio::test]
async fn shell_resume_target_bypasses_provider_binding_wait() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.resume_provider_binding_timeout = PROVIDER_WAIT_LONG_TIMEOUT;
    host.resume_provider_binding_poll_interval = PROVIDER_WAIT_TEST_POLL_INTERVAL;
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let runtime = live_resume_runtime(&host, &transaction, BootstrapDelivery::NotRequired);

    let binding = tokio::time::timeout(
        PROVIDER_WAIT_IMMEDIATE_BOUND,
        host.await_authoritative_resume_binding(&manifest, &transaction, &runtime),
    )
    .await
    .expect(PROVIDER_SHELL_WAIT_TIMEOUT_DETAIL)
    .unwrap();

    assert!(binding.is_none());
}

#[tokio::test]
async fn committed_provider_completion_requires_a_persisted_receipt() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    fixture.publish_binding(&fixture.planned);
    let mut committed = fixture.manifest.clone();
    committed.recovery_state = fixture.transaction.target.completed_state();
    fixture
        .host
        .write_namespace_json(
            &fixture.host.paths.manifest(fixture.transaction.terminal_id),
            &committed,
        )
        .unwrap();

    let result = tokio::time::timeout(
        PROVIDER_WAIT_IMMEDIATE_BOUND,
        fixture
            .host
            .finish_committed_resume(committed, &fixture.transaction),
    )
    .await
    .expect(PROVIDER_POST_COMMIT_TIMEOUT_DETAIL);

    assert_resume_failure_retains(&fixture.host, &fixture.transaction, result);
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_NO_SCAN_CALLS
    );
    fixture.assert_authority_files_retained();
}
