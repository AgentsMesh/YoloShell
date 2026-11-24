#[test]
fn pending_provider_resume_with_a_foreign_authority_clears_and_retires() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let terminal_id = fixture.transaction.terminal_id;
    let runtime_path = fixture.host.paths.runtime_record(terminal_id);
    let mut foreign = fixture.runtime.clone();
    foreign.runtime_epoch = Uuid::new_v4();
    foreign.worker_pid = fixture.runtime.worker_pid.saturating_add(1);
    foreign.worker_start_time = fixture.runtime.worker_start_time.saturating_add(1);
    fixture
        .host
        .write_namespace_json(&runtime_path, &foreign)
        .unwrap();
    fixture.publish_binding(&fixture.planned);
    fixture
        .host
        .write_namespace_json(&runtime_path, &fixture.runtime)
        .unwrap();
    let mut manifest = fixture.host.load_durable_manifest(terminal_id).unwrap();

    assert!(
        fixture
            .host
            .settle_pending_provider_resume(&mut manifest, &fixture.transaction)
            .unwrap()
    );

    let manifest = fixture.host.load_durable_manifest(terminal_id).unwrap();
    assert_eq!(manifest.recovery_state, RecoveryState::Uncertain);
    assert!(!fixture.host.resume_transaction_needs_reconcile(&manifest));
    assert!(!fixture.binding_path().exists());
    assert!(
        fixture
            .host
            .load_resume_transaction(terminal_id)
            .unwrap()
            .is_none()
    );
    assert!(!fixture.host.paths.worker_launch(terminal_id).exists());
}

#[tokio::test]
async fn a_terminal_resume_failure_retires_the_transaction() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let terminal_id = fixture.transaction.terminal_id;

    fixture
        .host
        .retire_terminal_resume_failure(terminal_id)
        .await;

    let manifest = fixture.host.load_durable_manifest(terminal_id).unwrap();
    assert_eq!(manifest.recovery_state, RecoveryState::Uncertain);
    assert!(!fixture.host.resume_transaction_needs_reconcile(&manifest));
    assert!(
        fixture
            .host
            .load_resume_transaction(terminal_id)
            .unwrap()
            .is_none()
    );
    assert!(!fixture.host.paths.worker_launch(terminal_id).exists());
}
