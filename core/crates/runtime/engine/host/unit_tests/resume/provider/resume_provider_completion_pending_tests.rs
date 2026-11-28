use crate::host::tests::*;
#[tokio::test]
async fn failed_provider_scan_remains_restoring_and_can_retry() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    fixture.publish_binding(&fixture.resumed_binding());
    fixture.set_scan(ScriptedProviderScan::Failed);

    let pending = finish_provider_auth_only(
        &fixture.host,
        fixture.manifest.clone(),
        &fixture.transaction,
        &fixture.runtime,
    )
    .await;

    assert_provider_completion_pending_retains(&fixture.host, &fixture.transaction, pending);
    fixture.set_scan(ScriptedProviderScan::Stable(vec![fixture.valid_process()]));
    let manifest = fixture
        .host
        .load_durable_manifest(fixture.transaction.terminal_id)
        .unwrap();
    let view = finish_provider_ready(
        &fixture.host,
        manifest,
        &fixture.transaction,
        &fixture.runtime,
    )
    .await
    .unwrap();
    assert_eq!(view.manifest.recovery_state, RecoveryState::Resumed);
}

#[tokio::test]
async fn timed_out_provider_scan_releases_the_retry_path() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    fixture.publish_binding(&fixture.resumed_binding());
    let blocking = BlockingProviderScan::new(crate::agent::FencedDescendants::RootChanged);
    fixture.set_scan(ScriptedProviderScan::Blocking(blocking.clone()));

    let (pending, ()) = tokio::join!(
        finish_provider_auth_only(
            &fixture.host,
            fixture.manifest.clone(),
            &fixture.transaction,
            &fixture.runtime,
        ),
        blocking.await_entry_with(PROVIDER_WAIT_LONG_TIMEOUT),
    );

    assert_provider_completion_pending_retains(&fixture.host, &fixture.transaction, pending);
    blocking.await_settlement().await;
    fixture.set_scan(ScriptedProviderScan::Stable(vec![fixture.valid_process()]));
    let manifest = fixture
        .host
        .load_durable_manifest(fixture.transaction.terminal_id)
        .unwrap();
    let view = finish_provider_ready(
        &fixture.host,
        manifest,
        &fixture.transaction,
        &fixture.runtime,
    )
    .await
    .unwrap();
    assert_eq!(view.manifest.recovery_state, RecoveryState::Resumed);
}

#[tokio::test]
async fn contradicted_provider_process_marks_the_resume_uncertain() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_LONG_TIMEOUT);
    fixture.publish_binding(&fixture.resumed_binding());
    let mut process = fixture.valid_process();
    process.argv = vec![crate::agent::codex::constants::EXECUTABLE.to_owned()];
    fixture.set_scan(ScriptedProviderScan::Stable(vec![process]));

    let contradicted = finish_provider_auth_only(
        &fixture.host,
        fixture.manifest.clone(),
        &fixture.transaction,
        &fixture.runtime,
    )
    .await;

    assert_resume_failure_retains(&fixture.host, &fixture.transaction, contradicted);
    fixture.assert_authority_files_retained();
}
