use crate::host::tests::*;
#[tokio::test]
async fn previous_binding_is_pending_until_the_successor_hook_publishes() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    fixture.publish_raw_binding(&fixture.planned);
    assert_eq!(
        fixture.host.observe_resume_provider_binding(
            &fixture.manifest, &fixture.transaction, &fixture.runtime,
        ).unwrap(),
        ProviderBindingObservation::Pending,
    );
    assert_eq!(fixture.process_scanner.calls(), COMPLETION_PROVIDER_NO_SCAN_CALLS);
}

#[tokio::test]
async fn provider_wait_rotates_the_planned_pid_within_one_resume() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_SUCCESS_TIMEOUT);
    fixture.publish_binding(&fixture.planned);
    assert_eq!(
        fixture
            .host
            .observe_resume_provider_binding(
                &fixture.manifest,
                &fixture.transaction,
                &fixture.runtime,
            )
            .unwrap(),
        ProviderBindingObservation::Pending
    );

    let wait = fixture.host.await_authoritative_resume_binding(
        &fixture.manifest,
        &fixture.transaction,
        &fixture.runtime,
    );
    let publication = async {
        assert_eq!(
            fixture.process_scanner.calls(),
            COMPLETION_PROVIDER_NO_SCAN_CALLS
        );
        fixture.publish_binding(&fixture.resumed_binding());
    };
    let (completed, ()) = tokio::join!(biased; wait, publication);
    let authority = completed.unwrap().unwrap().provider_completion.unwrap();

    assert_eq!(authority.binding, fixture.resumed_binding());
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_EXPECTED_SCAN_CALLS
    );
    fixture.assert_authority_files_retained();
}

#[tokio::test]
async fn provider_wait_distinguishes_missing_and_stale_timeouts() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let missing_started = tokio::time::Instant::now();
    let missing = fixture
        .host
        .await_authoritative_resume_binding(
            &fixture.manifest,
            &fixture.transaction,
            &fixture.runtime,
        )
        .await
        .unwrap_err();
    assert!(missing_started.elapsed() >= PROVIDER_WAIT_TEST_TIMEOUT);
    assert!(
        missing
            .to_string()
            .contains(RESUME_PROVIDER_BINDING_MISSING)
    );
    assert!(missing.is_pending());

    fixture.publish_binding(&fixture.planned);
    let stale_started = tokio::time::Instant::now();
    let stale = fixture
        .host
        .await_authoritative_resume_binding(
            &fixture.manifest,
            &fixture.transaction,
            &fixture.runtime,
        )
        .await
        .unwrap_err();
    assert!(stale_started.elapsed() >= PROVIDER_WAIT_TEST_TIMEOUT);
    assert!(stale.to_string().contains(RESUME_PROVIDER_BINDING_PENDING));
    assert!(stale.is_pending());
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_NO_SCAN_CALLS
    );
    fixture.assert_authority_files_retained();
}

#[tokio::test]
async fn provider_wait_rejects_stable_field_mutation_without_waiting() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    let mut changed = fixture.resumed_binding();
    changed.executable = fixture
        .directory
        .path()
        .join(crate::agent::codex::constants::EXECUTABLE)
        .to_string_lossy()
        .into_owned();
    fixture.publish_binding(&changed);

    assert_eq!(
        fixture
            .host
            .observe_resume_provider_binding(
                &fixture.manifest,
                &fixture.transaction,
                &fixture.runtime,
            )
            .unwrap(),
        ProviderBindingObservation::Rejected
    );
    let error = await_immediate_provider_rejection(&fixture).await;
    assert!(error.to_string().contains(RESUME_PROVIDER_BINDING_MISMATCH));
    assert!(!error.is_pending());
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_NO_SCAN_CALLS
    );
    fixture.assert_authority_files_retained();
}

#[tokio::test]
async fn provider_wait_rejects_non_resume_codex_process_immediately() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    fixture.publish_binding(&fixture.resumed_binding());
    let mut process = fixture.valid_process();
    process.argv = vec![crate::agent::codex::constants::EXECUTABLE.to_owned()];
    fixture.set_scan(ScriptedProviderScan::Stable(vec![process]));

    assert_eq!(
        fixture
            .host
            .observe_resume_provider_binding(
                &fixture.manifest,
                &fixture.transaction,
                &fixture.runtime,
            )
            .unwrap(),
        ProviderBindingObservation::Candidate(fixture.resumed_binding())
    );
    let error = await_immediate_provider_rejection(&fixture).await;
    assert!(error.to_string().contains(RESUME_PROVIDER_BINDING_MISMATCH));
    assert!(!error.is_pending());
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_EXPECTED_SCAN_CALLS
    );
    fixture.assert_authority_files_retained();
}

#[tokio::test]
async fn provider_wait_rejects_wrong_yolo_environment_immediately() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    fixture.publish_binding(&fixture.resumed_binding());
    let mut process = fixture.valid_process();
    process.environment.as_mut().unwrap().insert(
        crate::agent::constants::YOLO_RUNTIME_EPOCH_ENV.to_owned(),
        COMPLETION_PROVIDER_OTHER_SESSION_ID.to_owned(),
    );
    fixture.set_scan(ScriptedProviderScan::Stable(vec![process]));

    let error = await_immediate_provider_rejection(&fixture).await;
    assert!(error.to_string().contains(RESUME_PROVIDER_BINDING_MISMATCH));
    assert!(!error.is_pending());
    assert_eq!(
        fixture.process_scanner.calls(),
        COMPLETION_PROVIDER_EXPECTED_SCAN_CALLS
    );
    fixture.assert_authority_files_retained();
}
