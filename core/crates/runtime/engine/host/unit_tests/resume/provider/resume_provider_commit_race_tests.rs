use crate::host::tests::*;
#[test]
fn provider_completion_commit_accepts_benign_status_progress() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let observed = fixture.resumed_binding();
    let authority = ProviderResumeAuthority::new(
        &fixture.transaction,
        &fixture.runtime,
        observed.clone(),
        observed.updated_at_ms,
    )
    .unwrap();
    let mut current = observed;
    current.state = crate::agent::AgentState::WaitingForInput;
    current.updated_at_ms = current
        .updated_at_ms
        .saturating_add(COMPLETION_PROVIDER_TIMESTAMP_INCREMENT);
    fixture.publish_binding(&current);

    let completed = fixture
        .host
        .record_provider_completion(
            &fixture.transaction,
            authority.clone(),
            &fixture.manifest,
            &fixture.runtime,
        )
        .unwrap();

    assert_eq!(completed.provider_completion, Some(authority));
    assert_eq!(
        read_json::<ResumeTransaction>(
            &fixture
                .host
                .paths
                .resume_transaction(fixture.transaction.terminal_id)
        )
        .unwrap(),
        completed
    );
}

#[test]
fn provider_completion_commit_rejects_authority_identity_and_availability_drift() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let observed = fixture.resumed_binding();
    let mut active = observed.clone();
    active.state = crate::agent::AgentState::WaitingForInput;
    active.updated_at_ms = active
        .updated_at_ms
        .saturating_add(COMPLETION_PROVIDER_TIMESTAMP_INCREMENT);
    assert!(provider_binding_commit_matches(&observed, &active));

    let mutations: &[fn(&mut AgentBinding)] = &[
        |value| value.process_pid = Some(COMPLETION_PROVIDER_PREVIOUS_PID),
        |value| value.process_start_time = Some(COMPLETION_PROVIDER_PREVIOUS_START_TIME),
        |value| {
            value
                .provider_session_id
                .push_str(COMPLETION_PROVIDER_CHANGED_FIELD)
        },
        |value| value.state = crate::agent::AgentState::Exited,
        |value| value.recovery_availability = crate::agent::AgentRecoveryAvailability::Unavailable,
        |value| value.updated_at_ms = value.detected_at_ms.saturating_sub(1),
    ];
    for mutate in mutations {
        let mut changed = active.clone();
        mutate(&mut changed);
        assert!(!provider_binding_commit_matches(&observed, &changed));
    }
}

#[test]
fn unresolved_activity_does_not_block_an_authenticated_resume_completion() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let observed = fixture.resumed_binding();
    let mut current = observed.clone();
    current.state = crate::agent::AgentState::Active;
    current.updated_at_ms += COMPLETION_PROVIDER_TIMESTAMP_INCREMENT;
    assert!(provider_binding_commit_matches(&observed, &current));
    let authority = ProviderResumeAuthority::new(
        &fixture.transaction,
        &fixture.runtime,
        current.clone(),
        current.updated_at_ms,
    )
    .unwrap();
    fixture.publish_binding(&current);
    fixture
        .host
        .record_provider_completion(
            &fixture.transaction,
            authority,
            &fixture.manifest,
            &fixture.runtime,
        )
        .unwrap();
}

#[test]
fn claude_provider_completion_requires_native_first_event_receipt() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let mut binding = fixture.resumed_binding();
    binding.kind = crate::agent::AgentKind::ClaudeCode;
    assert!(
        !fixture
            .host
            .provider_startup_receipt_ready(&fixture.runtime, &binding)
            .unwrap()
    );
    let record = publish_native_claude_receipt(&fixture, &binding);
    binding = record.recovery_binding(true);
    assert!(
        fixture
            .host
            .provider_startup_receipt_ready(&fixture.runtime, &binding)
            .unwrap()
    );
    let mut changed_runtime = fixture.runtime.clone();
    changed_runtime.runtime_epoch = Uuid::new_v4();
    assert!(
        !fixture
            .host
            .provider_startup_receipt_ready(&changed_runtime, &binding)
            .unwrap()
    );
    let store =
        crate::agent::claude::generation::native::ClaudeNativeGenerationStore::from_binding_path(
            &fixture.binding_path(),
        )
        .unwrap();
    store
        .revoke_in_namespace(&fixture.host.namespace, &record, || Ok(()))
        .unwrap();
    assert!(
        !fixture
            .host
            .provider_startup_receipt_ready(&fixture.runtime, &binding)
            .unwrap()
    );
}
